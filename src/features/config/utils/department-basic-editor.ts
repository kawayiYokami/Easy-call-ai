import type { DepartmentConfig } from "../../../types/app";
import { normalizeDepartmentChildIds } from "./department-graph";

export function buildDepartmentBasicSnapshot(
  departments: DepartmentConfig[] | null | undefined,
): string {
  return JSON.stringify(
    (departments || []).map((item) => ({
      id: item.id,
      name: item.name,
      summary: item.summary,
      guide: item.guide,
      apiConfigId: item.apiConfigId,
      apiConfigIds: [...item.apiConfigIds],
      modelFailureFallbackEnabled: item.modelFailureFallbackEnabled,
      agentIds: [...item.agentIds],
      orderIndex: item.orderIndex,
      permissionControl: item.permissionControl,
    })),
  );
}

export function departmentBasicComparableSnapshot(department: DepartmentConfig): string {
  return JSON.stringify({
    id: department.id,
    name: department.name,
    summary: department.summary,
    guide: department.guide,
    apiConfigId: department.apiConfigId,
    apiConfigIds: [...department.apiConfigIds],
    modelFailureFallbackEnabled: department.modelFailureFallbackEnabled,
    agentIds: [...department.agentIds],
    orderIndex: department.orderIndex,
    permissionControl: department.permissionControl,
  });
}

export function mergeDepartmentChildIdsFromSource(
  targetDepartments: DepartmentConfig[] | null | undefined,
  sourceDepartments: DepartmentConfig[] | null | undefined,
): DepartmentConfig[] {
  const targetIds = new Set(
    (targetDepartments || [])
      .map((department) => String(department.id || "").trim())
      .filter(Boolean),
  );
  const sourceChildIdsById = new Map(
    (sourceDepartments || []).map((department) => {
      const id = String(department.id || "").trim();
      return [
        id,
        normalizeDepartmentChildIds(department.childDepartmentIds, id)
          .filter((childId) => targetIds.has(childId)),
      ] as const;
    }),
  );
  return (targetDepartments || []).map((department) => {
    const id = String(department.id || "").trim();
    const sourceChildIds = sourceChildIdsById.get(id);
    return {
      ...department,
      childDepartmentIds: sourceChildIds
        ? [...sourceChildIds]
        : normalizeDepartmentChildIds(department.childDepartmentIds, id),
    };
  });
}
