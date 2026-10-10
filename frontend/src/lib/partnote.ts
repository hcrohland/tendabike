import { mapableState, stateValues } from "./mapable.svelte";
import { checkStatus, handleError, myfetch } from "./store";

export class PartNote {
  id?: number;
  part: number;
  name: string;
  mime?: string;
  filename?: string;
  size?: number;
  created: Date;

  constructor(data: any) {
    this.id = data.id;
    this.part = data.part;
    this.name = data.name || "";
    this.mime = data.mime;
    this.filename = data.filename;
    this.size = data.size;
    this.created = data.created ? new Date(data.created) : new Date();
  }

  hasFile() {
    return this.mime != null;
  }

  hasImage() {
    return this.mime?.startsWith("image/");
  }

  fileUrl() {
    return this.id ? `/api/part/notes/${this.id}/file` : undefined;
  }

  /// The writes answer 204 (201 for creates): the change rides the stream
  /// frame, so the handlers only await the write.
  async updateName(name: string) {
    return await myfetch(`/api/part/notes/${this.id}`, "PUT", { name }).catch(
      handleError,
    );
  }

  async updateFile(name: string, file: File) {
    const formData = new FormData();
    formData.append("name", name);
    formData.append("file", file, file.name);
    return await fetch(`/api/part/notes/${this.id}/file`, {
      method: "PUT",
      credentials: "include",
      body: formData,
    })
      .then(checkStatus)
      .catch(handleError);
  }

  async removeFile() {
    return await myfetch(`/api/part/notes/${this.id}/file`, "DELETE").catch(
      handleError,
    );
  }

  async delete() {
    return await myfetch(`/api/part/notes/${this.id}`, "DELETE").catch(
      handleError,
    );
  }
}

export const partNotes = mapableState("id", (p) => new PartNote(p));

export function notes_for_part(partId: number): PartNote[] {
  return stateValues(partNotes).filter((n) => n.part == partId);
}

export async function createTextNote(part: number, name: string) {
  return await myfetch(`/api/part/${part}/notes`, "POST", { name }).catch(
    handleError,
  );
}

export function fmtSize(bytes: number): string {
  if (bytes < 1024) return bytes + " B";
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
  return (bytes / (1024 * 1024)).toFixed(1) + " MB";
}

export async function createFileNote(part: number, name: string, file: File) {
  const formData = new FormData();
  formData.append("name", name);
  formData.append("file", file);
  return await fetch(`/api/part/${part}/notes/file`, {
    method: "POST",
    credentials: "include",
    body: formData,
  })
    .then(checkStatus)
    .catch(handleError);
}
