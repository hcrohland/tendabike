import { handleError, myfetch, updateSummary } from "./store";
import { mapable } from "./mapable";

export class Garage {
  id?: number;
  owner: number;
  owner_firstname: string;
  owner_name: string;
  name: string;
  description?: string;
  created_at: Date;

  constructor(data: any) {
    this.id = data.id;
    this.owner = data.owner;
    this.owner_firstname = data.owner_firstname || "";
    this.owner_name = data.owner_name || "";
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
    return await myfetch(`/api/garage/${this.id}/parts`, "POST", {
      part_id: partId,
    })
      .then((data) => updateSummary(data))
      .catch(handleError);
  }

  async unregisterPart(partId: number) {
    return await myfetch(`/api/garage/${this.id}/parts/${partId}`, "DELETE")
      .then((data) => updateSummary(data))
      .catch(handleError);
  }

  async getParts(): Promise<number[]> {
    return await myfetch(`/api/garage/${this.id}/parts`, "GET").catch(
      handleError,
    );
  }

  async requestSubscription(message?: string) {
    return await myfetch("/api/garage/subscriptions", "POST", {
      garage_id: this.id,
      message: message,
    }).catch(handleError);
  }
}

export const garages = mapable<Garage>("id", (data) => new Garage(data));
