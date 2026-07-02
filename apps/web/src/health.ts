export type ApiHealth = {
  status: string;
  service: string;
};

export function formatHealthStatus(health: ApiHealth): string {
  return health.status === "ok" ? "API online" : "API degraded";
}
