export type ServiceStatus = "ok" | "starting" | "unavailable";

export interface HealthResponse {
  status: "ok" | "degraded";
  database: "ok" | "unavailable";
  lightning: ServiceStatus;
}
