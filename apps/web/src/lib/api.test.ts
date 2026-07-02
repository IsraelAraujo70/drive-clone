import { afterEach, describe, expect, it, vi } from "vitest";
import { API_BASE_URL, ApiError, api } from "./api";

function jsonResponse(status: number, body: unknown) {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("api client", () => {
  it("posts signup input as JSON", async () => {
    const fetchMock = vi.fn().mockResolvedValue(
      jsonResponse(201, { user: { email: "a@b.co" }, token: "tok" }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await api.signup({
      email: "a@b.co",
      password: "password123",
      display_name: "A",
    });

    expect(result.token).toBe("tok");
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`${API_BASE_URL}/auth/signup`);
    expect(init.method).toBe("POST");
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(init.body)).toEqual({
      email: "a@b.co",
      password: "password123",
      display_name: "A",
    });
  });

  it("sends the bearer token on authenticated calls", async () => {
    const fetchMock = vi.fn().mockResolvedValue(jsonResponse(200, { email: "a@b.co" }));
    vi.stubGlobal("fetch", fetchMock);

    await api.me("secret-token");

    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe(`${API_BASE_URL}/auth/me`);
    expect(init.headers.Authorization).toBe("Bearer secret-token");
  });

  it("throws ApiError with the server error code and message", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(
        jsonResponse(401, { error: "invalid_credentials", message: "Invalid email or password" }),
      ),
    );

    const error = await api
      .login({ email: "a@b.co", password: "wrong" })
      .catch((caught: unknown) => caught);

    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).status).toBe(401);
    expect((error as ApiError).code).toBe("invalid_credentials");
    expect((error as ApiError).message).toBe("Invalid email or password");
  });

  it("handles empty 204 responses", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(null, { status: 204 })));
    await expect(api.logout("secret-token")).resolves.toBeUndefined();
  });

  it("falls back to a generic error on non-JSON failures", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue(new Response("boom", { status: 500 })),
    );

    const error = await api.health().catch((caught: unknown) => caught);
    expect(error).toBeInstanceOf(ApiError);
    expect((error as ApiError).code).toBe("unknown_error");
  });
});
