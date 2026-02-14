import { expect, test } from "@playwright/test";
import { API_BASE } from "./helpers";

test.describe("API Endpoints", () => {
  test("GET /api/auth/status returns auth info", async ({ request }) => {
    const res = await request.get(`${API_BASE}/api/auth/status`);
    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(body).toHaveProperty("configured");
    expect(body).toHaveProperty("type");
  });

  test("GET /api/status returns system status", async ({ request }) => {
    const res = await request.get(`${API_BASE}/api/status`);
    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(body).toHaveProperty("auth");
    expect(body).toHaveProperty("session");
    expect(body).toHaveProperty("version");
    expect(body).toHaveProperty("mcp_servers");
    expect(body).toHaveProperty("drones");
  });

  test("GET /api/registry/projects returns project list", async ({ request }) => {
    const res = await request.get(`${API_BASE}/api/registry/projects`);
    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(Array.isArray(body)).toBe(true);

    if (body.length > 0) {
      const project = body[0];
      expect(project).toHaveProperty("id");
      expect(project).toHaveProperty("name");
      expect(project).toHaveProperty("path");
    }
  });

  test("GET /api/chat/sessions returns session list", async ({ request }) => {
    const res = await request.get(`${API_BASE}/api/chat/sessions`);
    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(Array.isArray(body)).toBe(true);

    if (body.length > 0) {
      const session = body[0];
      expect(session).toHaveProperty("id");
      expect(session).toHaveProperty("status");
    }
  });

  test("GET /api/models returns model list", async ({ request }) => {
    const res = await request.get(`${API_BASE}/api/models`);
    expect(res.ok()).toBe(true);
    const body = await res.json();
    expect(Array.isArray(body)).toBe(true);
    expect(body.length).toBeGreaterThan(0);
  });

  test("POST /api/registry/projects validates input", async ({ request }) => {
    const res = await request.post(`${API_BASE}/api/registry/projects`, {
      data: { name: "", path: "" },
    });
    expect(res.ok()).toBe(false);
    expect(res.status()).toBeGreaterThanOrEqual(400);
  });

  test("POST /api/registry/projects rejects non-existent path", async ({ request }) => {
    const res = await request.post(`${API_BASE}/api/registry/projects`, {
      data: { name: "fake", path: "/nonexistent/path/12345" },
    });
    expect(res.ok()).toBe(false);
    const body = await res.json();
    expect(body.error).toContain("does not exist");
  });
});

test.describe("SSE Endpoints", () => {
  test("GET /api/events SSE endpoint is reachable", async ({ page }) => {
    // Use page.evaluate to test SSE connectivity since request API
    // doesn't handle streaming well. Just verify the endpoint exists.
    const status = await page.evaluate(async () => {
      const res = await fetch("http://localhost:3333/api/events");
      return res.status;
    });
    expect(status).toBe(200);
  });
});
