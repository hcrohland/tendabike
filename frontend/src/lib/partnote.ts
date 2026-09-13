import { type Map, filterValues, mapable } from "./mapable";
import { checkStatus, handleError, myfetch } from "./store";

export type NoteKind = "text" | "file";

export class PartNote {
  id?: number;
  part: number;
  kind: NoteKind;
  name: string;
  mime?: string;
  filename?: string;
  size?: number;
  created: Date;

  constructor(data: any) {
    this.id = data.id;
    this.part = data.part;
    this.kind = data.kind;
    this.name = data.name || "";
    this.mime = data.mime;
    this.filename = data.filename;
    this.size = data.size;
    this.created = data.created ? new Date(data.created) : new Date();
  }

  isImage() {
    return this.kind === "file" && this.mime?.startsWith("image/");
  }

  fileUrl() {
    return this.id ? `/api/part/notes/${this.id}/file` : undefined;
  }

  async updateName(name: string) {
    return await myfetch(`/api/part/notes/${this.id}`, "PUT", { name })
      .then((data) => partNotes.updateMap([data]))
      .catch(handleError);
  }

  async updateFile(name: string, file: File) {
    const formData = new FormData();
    formData.append("name", name);
    formData.append("file", file, file.name);
    const res = await fetch(`/api/part/notes/${this.id}/file`, {
      method: "PUT",
      credentials: "include",
      body: formData,
    })
      .then(checkStatus)
      .catch(handleError);
    if (!res) return res;
    partNotes.updateMap([res]);
    return res;
  }

  async removeFile() {
    return await myfetch(`/api/part/notes/${this.id}/file`, "DELETE")
      .then((data) => partNotes.updateMap([data]))
      .catch(handleError);
  }

  async delete() {
    return await myfetch(`/api/part/notes/${this.id}`, "DELETE")
      .then((data) => partNotes.deleteItem(data))
      .catch(handleError);
  }
}

export const partNotes = mapable("id", (p) => new PartNote(p));

export function notes_for_part(
  notes: Map<PartNote>,
  partId: number,
): PartNote[] {
  return filterValues(notes, (n) => n.part == partId);
}

export async function createTextNote(part: number, name: string) {
  return await myfetch(`/api/part/${part}/notes`, "POST", { name })
    .then((data) => {
      partNotes.updateMap([data]);
      return data;
    })
    .catch(handleError);
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
  const res = await fetch(`/api/part/${part}/notes/file`, {
    method: "POST",
    credentials: "include",
    body: formData,
  })
    .then(checkStatus)
    .catch(handleError);
  if (!res) return res;
  partNotes.updateMap([res]);
  return res;
}
