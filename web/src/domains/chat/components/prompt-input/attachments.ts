import type { ImageAttachment } from "../../types";

// ── Constants ────────────────────────────────────────────────────────────────

export const ACCEPTED_IMAGE_TYPES = [
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
  "image/svg+xml",
];

// ── Unique ID generator ─────────────────────────────────────────────────────

let idCounter = 0;
export function uniqueId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${Date.now()}-${idCounter}`;
}

// ── File to ImageAttachment ──────────────────────────────────────────────────

export function fileToAttachment(file: File): Promise<ImageAttachment> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error(`Expected data URL string for ${file.name}`));
        return;
      }
      resolve({
        id: uniqueId("img"),
        dataUrl: result,
        mimeType: file.type,
        name: file.name,
      });
    };
    reader.onerror = () => reject(new Error(`Failed to read ${file.name}`));
    reader.readAsDataURL(file);
  });
}
