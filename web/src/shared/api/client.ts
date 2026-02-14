class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

async function request(path: string, init?: RequestInit): Promise<Response> {
  const res = await fetch(path, init);
  if (!res.ok) throw new ApiError(res.status, await res.text());
  return res;
}

/**
 * JSON parse at the API boundary — the only place `as T` is acceptable.
 * Callers trust the server to return the expected shape. Runtime validation
 * happens at the domain layer (type guards, discriminated unions).
 */
async function parseJson<T>(res: Response): Promise<T> {
  // eslint-disable-next-line @typescript-eslint/consistent-type-assertions -- API boundary
  const data: unknown = await res.json();
  return data as T;
}

function jsonInit(method: string, body?: unknown, signal?: AbortSignal): RequestInit {
  return {
    method,
    headers: body ? { "Content-Type": "application/json" } : undefined,
    body: body ? JSON.stringify(body) : undefined,
    signal,
  };
}

class ApiClient {
  async get<T>(path: string, signal?: AbortSignal): Promise<T> {
    const res = await request(path, { signal });
    return parseJson<T>(res);
  }

  async post<T>(path: string, body?: unknown, signal?: AbortSignal): Promise<T> {
    const res = await request(path, jsonInit("POST", body, signal));
    return parseJson<T>(res);
  }

  async postVoid(path: string, body?: unknown, signal?: AbortSignal): Promise<void> {
    await request(path, jsonInit("POST", body, signal));
  }

  async put<T>(path: string, body?: unknown): Promise<T> {
    const res = await request(path, jsonInit("PUT", body));
    return parseJson<T>(res);
  }

  async patch<T>(path: string, body?: unknown): Promise<T> {
    const res = await request(path, jsonInit("PATCH", body));
    return parseJson<T>(res);
  }

  async delete(path: string): Promise<void> {
    await request(path, { method: "DELETE" });
  }
}

export const apiClient = new ApiClient();
export { ApiError };
