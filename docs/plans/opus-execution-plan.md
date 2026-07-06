# Plano de execução — fase final do drive-clone

Executor: agente Opus. Autor do plano: Fable (tech lead). Data: 2026-07-06.

Este documento é autocontido. Todas as decisões de arquitetura já foram tomadas — não reabra decisões, não pergunte, execute. Se encontrar contradição real entre este plano e o código, pare e reporte BLOCKED com evidência.

## Contexto do repo (verificado em 2026-07-06)

- Backend Rust/Axum 0.8/sqlx 0.8 em `services/api`, arquitetura hexagonal: `domain/`, `application/` (use cases + `application/ports/`), `adapters/` (`http/`, `postgres/`, `object_storage/`), `bootstrap/`. DI em `bootstrap/state.rs`, rotas em `bootstrap/router.rs`.
- Migrations SQL em `services/api/migrations/0001..0006_*.sql`, aplicadas no boot (`bootstrap/server.rs:25-28`). A próxima é `0007`.
- Frontend Next.js 16 em `apps/web`, cliente HTTP em `apps/web/lib/api.ts`, shell principal em `apps/web/components/drive-shell.tsx`.
- Testes: `cd services/api && cargo test` (integração com Postgres real via `#[sqlx::test]` em `tests/api_contract.rs`, unit com fakes em `src/application/tests.rs`); `cd apps/web && npm test` (vitest). Stack local: `make up` (docker-compose: api, web, postgres na porta 5433, minio).
- Storage: client S3 próprio com SigV4 manual em `adapters/object_storage/s3.rs`; trait em `application/ports/object_storage.rs`; fakes em `object_storage/fake.rs`. Object key = `{owner_id}/{uuid}`.
- Docs que DEVEM ser atualizadas junto com cada fase: `contracts/files.md`, `docs/api/README.md`, coleção Bruno em `docs/api/bruno/` (gerador: `scripts/generate-bruno-collection.mjs`), smoke scripts em `docs/evals/`.

## Regras para o executor

1. Uma fase = um commit (ou mais, se lógico), com testes e docs no mesmo commit. "Testes depois" não existe.
2. Siga o padrão existente: use case por arquivo em `application/files/`, SQL no `adapters/postgres/file_repository.rs`, handler em `adapters/http/file_routes.rs`, wiring em `bootstrap/state.rs` + `bootstrap/router.rs`. Copie o estilo dos use cases resumable (`application/files/resumable_upload.rs`) — é o código mais recente e mais idiomático do repo.
3. Todo teste de integração novo entra em `services/api/tests/api_contract.rs` seguindo o padrão `#[sqlx::test]` + `FakeObjectStorage`.
4. Rode `cargo test` e `npm test` ao fim de cada fase. Ambos verdes antes do commit.
5. Não use SDK AWS. Estenda o client S3 existente quando precisar de operação nova.
6. Status final por fase: DONE / BLOCKED, com evidência (saída de teste).

Ordem de execução: Fase 1 → 2 → 3 → 4. As fases 3 e 4 são independentes entre si, mas 2 depende de 1 (o purge da lixeira grava tombstone no change_log).

---

## Fase 1 — Sync contract (`GET /sync/changes`)

Fecha a exigência do challenge de sync sem desktop app. Feed de mudanças cursor-based com tombstones.

### Decisões tomadas

- **Cursor = sequência monotônica por owner**, não BIGSERIAL global. BIGSERIAL tem o problema clássico de visibilidade: transações commitam fora de ordem e um cursor pode pular seq ainda não commitada. Solução: coluna `change_seq BIGINT NOT NULL DEFAULT 0` em `users`; cada mutação faz `UPDATE users SET change_seq = change_seq + 1 WHERE id = $owner RETURNING change_seq` **na mesma transação** da mutação. O row lock em `users` serializa as escritas do owner até o commit, então a ordem de visibilidade == ordem de seq. Gap-free e estável por construção. Custo: serializa mutações do mesmo owner — aceitável para este produto.
- **Tombstone = linha no change_log com `op='delete'`**, não flag na entidade. O change_log sobrevive ao hard-delete da Fase 2.
- Trash (soft-delete) gera `op='delete'`; restore gera `op='upsert'`. Para um cliente de sync, item na lixeira = item removido do drive.
- Payload do upsert embute o snapshot da entidade (evita N+1 no cliente). Snapshot lido por join no momento do GET; se a entidade mudou de novo depois, tudo bem — o cliente receberá outra entrada adiante e o estado converge.

### Migration `0007_change_log.sql`

```sql
ALTER TABLE users ADD COLUMN change_seq BIGINT NOT NULL DEFAULT 0;

CREATE TABLE change_log (
    owner_id    UUID NOT NULL REFERENCES users(id),
    seq         BIGINT NOT NULL,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('file', 'folder')),
    entity_id   UUID NOT NULL,
    op          TEXT NOT NULL CHECK (op IN ('upsert', 'delete')),
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (owner_id, seq)
);
```

Sem índice extra: o PK cobre a query do feed.

### Backend

1. Port novo em `application/ports/files.rs` (ou trait dedicada): `record_change(&mut tx, owner_id, entity_type, entity_id, op) -> seq` e `list_changes(owner_id, after_seq, limit) -> Vec<ChangeLogEntry>`.
2. **Instrumentar toda mutação** dentro da transação existente no `file_repository.rs`. Levantamento dos pontos (confirme com grep antes de editar, a lista pode estar incompleta):
   - file: complete de upload direto e resumable (upsert — o `pending` não entra no feed), rename, move, `soft_delete_owned_file` (`file_repository.rs:973-994`, delete), restore (upsert).
   - folder: create, rename, move, `soft_delete_owned_folder_tree` (delete para CADA nó da árvore, dentro da CTE ou loop), restore de árvore (upsert para cada nó).
   - Métodos de repo que hoje não abrem transação explícita precisam passar a abrir para incluir o `record_change` atomicamente.
3. Use case `ListSyncChangesUseCase` em `application/files/sync_changes.rs`: valida `cursor >= 0`, `limit` clamp 1..=500 (default 100), retorna entradas + snapshot (join com `files`/`folders`; para tombstone só ids).
4. Endpoint `GET /sync/changes?cursor=<seq>&limit=<n>` (autenticado, escopo = owner). Response:

```json
{
  "changes": [
    { "seq": 42, "entity_type": "file", "op": "upsert", "entity_id": "…", "occurred_at": "…",
      "file": { "id": "…", "filename": "…", "parent_folder_id": null, "size_bytes": 1, "content_type": "…", "checksum_sha256": "…", "updated_at": "…" } },
    { "seq": 43, "entity_type": "folder", "op": "delete", "entity_id": "…", "occurred_at": "…" }
  ],
  "next_cursor": 43,
  "has_more": false
}
```

`next_cursor` = seq da última entrada retornada (ou o cursor recebido, se vazio). `has_more` = `changes.len() == limit`. Cursor inicial do cliente = `0`.

### Testes obrigatórios (api_contract.rs)

- Upload completo + rename + trash → feed retorna upsert, upsert, delete na ordem, com seqs crescentes sem buraco.
- Cursor: consumir com `limit=1` em 3 chamadas == consumir com `limit=100` em 1 chamada (mesmo conteúdo, mesma ordem).
- Restore de arquivo trashed → nova entrada upsert com seq maior.
- Trash recursivo de pasta com filhos → um delete por nó da árvore.
- Isolamento: mudanças do user A nunca aparecem no feed do user B.
- Cursor além do fim → `changes: []`, `next_cursor` == cursor enviado.
- Unit test do clamp de limit e cursor negativo → 400.

### Docs

`contracts/files.md` (nova seção Sync, remover/ajustar a menção "future" em `contracts/files.md:551-552`), `docs/api/README.md`, request Bruno em `docs/api/bruno/` (pasta `sync/`), smoke script `docs/evals/sync-changes-smoke.sh` no estilo dos existentes.

---

## Fase 2 — Worker real: purge da lixeira (30 dias), cleanup, reconciliação de quota

### Decisões tomadas

- **Worker = segundo binário do crate `services/api`**: converta o crate para lib + 2 bins (`src/bin/api.rs`, `src/bin/worker.rs`, ambos finos chamando `bootstrap`). Razão: o worker reusa ports, adapters, use cases e migrations; um crate separado duplicaria contrato interno sem ganho. `services/worker/README.md` passa a documentar isso e o comando de start (`cargo run --bin worker`). No Railway, é um serviço separado apontando pro mesmo repo com start command diferente — documente em `infra/railway/README.md`.
- **Semântica de quota**: arquivo na lixeira CONTINUA contando na quota (comportamento Google Drive). `storage_used_bytes` só decrementa no purge definitivo. Hoje o campo nunca decrementa (`file_repository.rs:585`, `:769` só somam) — o purge introduz o decremento.
- **Retenção**: env `TRASH_RETENTION_DAYS`, default `30`.
- **Loop do worker**: `tokio::time::interval` simples, um tick por job, intervalo via env `WORKER_INTERVAL_SECONDS` (default `300`). Sem lib de scheduler. Cada job em lote pequeno (`LIMIT 100`) com `FOR UPDATE SKIP LOCKED`, seguindo o padrão já usado em `file_repository.rs:598-634` — isso torna seguro rodar mais de uma instância.

### Jobs

1. **`expire_resumable_uploads`** — já existe (`ExpireResumableUploadsUseCase`, `resumable_upload.rs:342-385`). Só chamar no loop.
2. **`purge_trash`** — novo `PurgeTrashUseCase` em `application/files/purge_trash.rs`:
   - Seleciona `files` com `deleted_at < now() - retention` (batch, `FOR UPDATE SKIP LOCKED`).
   - Para cada um: `delete_object` no bucket → `DELETE FROM files` → `storage_used_bytes -= size_bytes` (com `GREATEST(0, …)`) → tombstone já existe no change_log desde o trash (Fase 1), não gravar outro.
   - Ordem importa: se o `delete_object` falhar, NÃO deleta a linha — loga e tenta no próximo tick. Se o objeto já não existir no bucket (404), trate como sucesso.
   - Pastas: `DELETE FROM folders` com `deleted_at` vencido só quando não restarem filhos (a purga de files esvazia primeiro; use a mesma CTE/ordem bottom-up do delete recursivo existente).
3. **`cleanup_orphan_objects`** — objeto no bucket sem linha `files` correspondente (upload que morreu entre `create_multipart` e o registro, ou purge interrompido após o `delete_object` falhar parcialmente). Requer `list_objects` no client S3 (não existe). Implemente `list_objects_v2` com paginação em `s3.rs` + no trait + fakes; o job compara keys `{owner}/{uuid}` contra `files.object_key` e deleta órfãos com idade > 24h (use `LastModified` do listing — não delete objeto recém-criado de upload em andamento).
4. **`reconcile_quota`** — `UPDATE users SET storage_used_bytes = COALESCE((SELECT SUM(size_bytes) FROM files WHERE owner_id = users.id AND state = 'complete'), 0)` — note que inclui trashed (semântica acima) e exclui pending/expired. Rode por batch de users; logue toda divergência encontrada (user_id, antes, depois) — divergência é bug em outro lugar, o log é a evidência.

### Operações S3 novas no client próprio (`s3.rs`)

`delete_object` (DELETE, SigV4) e `list_objects_v2` (GET com query `list-type=2`, parse XML de `Contents/Key/LastModified`, paginação por `NextContinuationToken`). Adicione ao trait `ObjectStorage`, ao `FakeObjectStorage` e ao `disabled.rs`. Unit tests inline no `s3.rs` seguindo os existentes (`s3.rs:431-472`).

### Correções embutidas nesta fase

- **`POST /files/uploads/cleanup-expired` (`file_routes.rs:97-104`)**: hoje qualquer usuário logado dispara limpeza global. Com o worker assumindo o job, **remova o endpoint** (e a request Bruno + doc correspondentes). Se algum smoke script em `docs/evals/` o usa, atualize o script.
- **Observabilidade mínima**: o worker loga por tick: job, itens processados, erros, duração. Use o logger que a API já usa; se não houver, `tracing` com `tracing-subscriber` (já é dep transitiva do Axum — confirme antes de adicionar).

### Testes obrigatórios

- Unit (fakes): purge deleta objeto + linha + decrementa quota; falha no `delete_object` mantém a linha; objeto 404 no bucket conta como sucesso; arquivo com `deleted_at` mais novo que a retenção não é tocado.
- Integração (`#[sqlx::test]`): trash → avança relógio (insira `deleted_at` no passado via SQL direto no teste) → roda use case de purge → linha sumiu, quota decrementou, `GET /sync/changes` ainda retorna o tombstone.
- Reconcile: corrompa `storage_used_bytes` via SQL, rode reconcile, valide correção.
- Orphan cleanup unit com `FakeObjectStorage`: órfão velho deletado, órfão recente preservado, objeto com linha correspondente preservado.

### Docs

`services/worker/README.md` reescrito (o que roda, envs, como iniciar), `infra/railway/README.md` (novo serviço), `docs/api/README.md` (endpoint removido), `.env.example` (`TRASH_RETENTION_DAYS`, `WORKER_INTERVAL_SECONDS`), checklist de produção em `docs/architecture.md`.

---

## Fase 3 — UX de uploads

### 3a. Desacoplar TTL da sessão resumable (backend, primeiro)

Hoje `state.rs:84-91` passa `presigned_url_ttl_seconds` como TTL da sessão inteira — 15 min mata qualquer upload grande. Nova env `RESUMABLE_UPLOAD_TTL_SECONDS`, default `86400` (24h), em `bootstrap/config.rs` + `.env.example` + `docker-compose.yml`. Presigned URL por parte continua com `PRESIGNED_URL_TTL_SECONDS`. Teste unit: sessão criada com TTL de 24h expira/não expira nos limites.

### 3b. Endpoint de pending uploads server-side

`GET /files/uploads/pending` (autenticado): lista uploads `state='pending' AND upload_kind='resumable' AND upload_expires_at > now()` do owner, com `file_id, filename, size_bytes, part_size_bytes, parts_received (count de upload_parts), expires_at, parent_folder_id, checksum_sha256`. Use case novo seguindo `GetUploadStatusUseCase` (`resumable_upload.rs:155`). Teste de integração: cria resumable, envia 1 parte, lista pending, finaliza, lista de novo (vazia). Bruno request + contracts + doc.

### 3c. Frontend

Arquivos: `apps/web/components/drive-shell.tsx` (upload em `:760-882`, painel de recovery em `:1225-1236`, `handleResumeUpload` `:884`), `apps/web/lib/resumableUploads.ts`, `apps/web/lib/api.ts`.

1. **Pending uploads vindos do servidor**: no mount do shell, chame `GET /files/uploads/pending` e faça merge com o `localStorage` (server é a fonte de verdade para existência/expiração; localStorage só acrescenta o `resumableUploadKey` que casa arquivo local → sessão). Sessão que o servidor não conhece mais sai do localStorage (`forgetUpload`).
2. **Progresso por parte**: o estado de upload passa a expor `{parts_done, parts_total, bytes_done}`; ao retomar, inicialize com as partes já registradas (vêm do status). UI: barra de progresso com `parts_done/parts_total` e percentual, rótulo "Retomado da parte N" quando aplicável.
3. **Mensagem de falha de storage**: hoje o erro do PUT presigned cai genérico. Distinga três casos na UI: falha de rede no PUT da parte ("Falha ao enviar a parte N — verifique a conexão e retome"), 4xx/5xx da API (mostrar `error.code` traduzido, padrão de `apps/web/lib/shareErrors.ts` — crie `uploadErrors.ts` análogo), sessão expirada ("Upload expirou — inicie novamente").
4. **Botão "Limpar expirados"** no painel de pending: remove do localStorage tudo com `expires_at` vencido e sessões que o servidor reporta como `expired` (a expiração server-side é o worker da Fase 2 / `state='expired'`).
5. Testes vitest: merge server+localStorage (server ganha), cálculo de progresso com partes retomadas, mapeamento de erros em `uploadErrors.test.ts`, limpeza de expirados. Atualize `apps/web/lib/resumableUploads.test.ts` para o merge.
6. Atualize `docs/evals/resumable-resume-ui-smoke.sh` para o novo fluxo.

---

## Fase 4 — Share links revogáveis

Fecha a última superfície aberta. Escopo mínimo deliberado: link de leitura para arquivo único, revogável, com expiração opcional. Sem senha, sem link de pasta, sem permissão de escrita — documente esses três como extensões futuras em `contracts/files.md`.

### Decisões tomadas

- **Token opaco de 32 bytes random (base64url), armazenado como hash SHA-256.** Nunca guardar o token em claro — vazamento de dump do banco não pode dar acesso aos arquivos. O token em claro só aparece na response do POST de criação.
- Download por link **não autenticado** reusa o fluxo de presigned URL de download existente.
- Revogação = `revoked_at`; linha nunca é deletada (auditoria).

### Migration `0008_share_links.sql`

```sql
CREATE TABLE share_links (
    id          UUID PRIMARY KEY,
    file_id     UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    created_by  UUID NOT NULL REFERENCES users(id),
    token_hash  BYTEA NOT NULL UNIQUE,
    expires_at  TIMESTAMPTZ,
    revoked_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX share_links_file_idx ON share_links (file_id);
```

### Endpoints

- `POST /files/{file_id}/share-links` (owner) body `{ "expires_in_seconds": 604800 | null }` → `{ "id", "token", "url", "expires_at" }`. `url` montada com env `PUBLIC_WEB_URL` (nova, `.env.example`), formato `{PUBLIC_WEB_URL}/s/{token}`.
- `GET /files/{file_id}/share-links` (owner) → lista sem token (só id, created_at, expires_at, revoked_at).
- `DELETE /files/{file_id}/share-links/{link_id}` (owner) → seta `revoked_at`.
- `GET /shared/links/{token}` (**público, sem auth**) → valida hash, não revogado, não expirado, arquivo `state='complete' AND deleted_at IS NULL` → `{ "filename", "size_bytes", "content_type", "download_url" }` (presigned GET). Token inválido/revogado/expirado/arquivo trashed → **404 uniforme** (não distinguir os casos — evita oracle).

### Frontend

No diálogo de share existente (onde vive o share por email em `drive-shell.tsx`): aba/seção "Link", botão criar (copia pro clipboard), lista de links ativos com revogar. Página pública `apps/web/app/s/[token]/page.tsx`: nome, tamanho, botão de download; 404 amigável para link inválido. Erros mapeados no padrão `shareErrors.ts`.

### Testes obrigatórios

- Integração: criar link → GET público baixa metadados + download_url funciona (FakeObjectStorage); revogar → 404; expirado → 404; arquivo na lixeira → 404; owner de outro arquivo não cria/lista/revoga (404/403 conforme padrão do repo); token errado → 404.
- Unit: hash do token (criar → validar), `expires_in_seconds` negativo/zero → 400.
- Vitest: página pública e mapeamento de erros.

### Docs

`contracts/files.md` (seção share links, mover de "Future Contracts" — `contracts/files.md:549` — para contrato real), `docs/api/README.md`, Bruno (`shares/` ou pasta nova `share-links/`), smoke `docs/evals/share-link-smoke.sh`.

---

## Critério de pronto global

- `cargo test` e `npm test` verdes; smoke scripts novos rodam contra `make up`.
- `contracts/`, `docs/api/README.md`, Bruno e `docs/evals/` refletem 100% da superfície final (nenhum endpoint sem doc, nenhuma doc de endpoint removido).
- README raiz e landing/docs não prometem nada não implementado (o repo já teve commit `e0e3e9a` exatamente para isso — mantenha a disciplina).
- Um commit por fase no mínimo, mensagem convencional, sem `--no-verify`.
- Relatório final por fase: DONE/BLOCKED + evidência (contagem de testes, saída relevante).
