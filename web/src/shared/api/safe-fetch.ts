// ── Discriminated result of a fetch operation ────────────────────────────────

type FetchOk = { ok: true; response: Response };
type FetchAborted = { ok: false; type: "aborted" };
type FetchNetworkError = { ok: false; type: "network"; message: string };
type FetchApiError = { ok: false; type: "api"; status: number; message: string };

export type FetchResult = FetchOk | FetchAborted | FetchNetworkError | FetchApiError;

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Extracts error message from a JSON response body, falling back to raw text */
async function extractErrorMessage(response: Response): Promise<string> {
  const text = await response.text();
  try {
    const parsed: unknown = JSON.parse(text);
    if (
      parsed !== null &&
      typeof parsed === "object" &&
      "error" in parsed &&
      typeof (parsed as Record<string, unknown>).error === "string"
    ) {
      return (parsed as Record<string, unknown>).error as string;
    }
  } catch {
    // Not JSON — use raw text
  }
  return text;
}

function isAbortError(err: unknown): boolean {
  return err instanceof DOMException && err.name === "AbortError";
}

// ── safeFetch ────────────────────────────────────────────────────────────────

/** Wraps fetch() and returns a discriminated union instead of throwing */
export async function safeFetch(url: string, init?: RequestInit): Promise<FetchResult> {
  let response: Response;
  try {
    response = await fetch(url, init);
  } catch (err) {
    if (isAbortError(err)) {
      return { ok: false, type: "aborted" };
    }
    const message = err instanceof Error ? err.message : "Network error";
    return { ok: false, type: "network", message };
  }

  if (!response.ok) {
    const message = await extractErrorMessage(response);
    return { ok: false, type: "api", status: response.status, message };
  }

  return { ok: true, response };
}
