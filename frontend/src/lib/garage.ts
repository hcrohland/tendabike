import { handleError, myfetch, updateSummary } from "./store";
import { mapable } from "./mapable";

export class Garage {
  id?: number;
  owner: number;
  name: string;
  description?: string;
  created_at: Date;

  constructor(data: any) {
    this.id = data.id;
    this.owner = data.owner;
    this.name = data.name || "";
    this.description = data.description;
    this.created_at = new Date(data.created_at);
  }

  async create() {
    return await myfetch("/api/garage", "POST", {
      name: this.name,
      description: this.description,
    })
      .then((data) => {
        garages.updateMap([data]);
        return new Garage(data);
      })
      .catch(handleError);
  }

  async update() {
    return await myfetch("/api/garage/" + this.id, "PUT", {
      name: this.name,
      description: this.description,
    })
      .then((data) => garages.updateMap([data]))
      .catch(handleError);
  }

  async delete() {
    return await myfetch("/api/garage/" + this.id, "DELETE")
      .then((data) => garages.deleteItem(data))
      .catch(handleError);
  }

  async registerPart(partId: number) {
    return await myfetch(`/api/garage/${this.id}/parts/${partId}`, "POST")
      .then(() => {
        // Optionally trigger a refresh of the summary
        updateSummary();
      })
      .catch(handleError);
  }

  async unregisterPart(partId: number) {
    return await myfetch(`/api/garage/${this.id}/parts/${partId}`, "DELETE")
      .then(() => {
        // Optionally trigger a refresh of the summary
        updateSummary();
      })
      .catch(handleError);
  }

  async getParts(): Promise<number[]> {
    return await myfetch(`/api/garage/${this.id}/parts`, "GET").catch(
      handleError,
    );
  }
}

export const garages = mapable<Garage>("id");
