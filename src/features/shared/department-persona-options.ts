import type { ApiConfigItem, DepartmentConfig, PersonaProfile } from "../../types/app";
import { resolveModelRoleApiConfigId } from "../config/utils/model-role-options";

export type DepartmentPersonaOption = {
  id: string;
  departmentId: string;
  agentId: string;
  departmentName: string;
  agentName: string;
  label: string;
  name: string;
  ownerAgentId: string;
  ownerName: string;
  providerName?: string;
  modelName?: string;
  apiConfigId?: string;
  childDepartmentIds?: string[];
  unavailable?: boolean;
};

type BuildDepartmentPersonaOptionsInput = {
  departments: DepartmentConfig[] | null | undefined;
  personas: PersonaProfile[] | null | undefined;
  apiConfigs: ApiConfigItem[] | null | undefined;
  assistantDepartmentApiConfigId?: string;
  toolReviewApiConfigId?: string | null;
};

function trimText(value: unknown): string {
  return String(value || "").trim();
}

function departmentPrimaryApiConfigId(department: DepartmentConfig | null | undefined): string {
  const ids = Array.isArray(department?.apiConfigIds)
    ? department.apiConfigIds.map(trimText).filter(Boolean)
    : [];
  if (ids.length > 0) return ids[0];
  return trimText(department?.apiConfigId);
}

function departmentConversationApiConfigId(
  department: DepartmentConfig,
  input: BuildDepartmentPersonaOptionsInput,
): string {
  const directId = departmentPrimaryApiConfigId(department);
  if (directId) {
    return resolveModelRoleApiConfigId(directId, {
      assistantDepartmentApiConfigId: trimText(input.assistantDepartmentApiConfigId),
      toolReviewApiConfigId: trimText(input.toolReviewApiConfigId),
    });
  }
  if (department.id === "assistant-department" || department.isBuiltInAssistant) {
    return trimText(input.assistantDepartmentApiConfigId);
  }
  return "";
}

export function departmentPersonaOptionId(departmentId: string, agentId: string): string {
  return `${trimText(departmentId)}::${trimText(agentId)}`;
}

export function buildDepartmentPersonaOptions(
  input: BuildDepartmentPersonaOptionsInput,
): DepartmentPersonaOption[] {
  const personas = new Map(
    (input.personas || [])
      .map((persona) => [trimText(persona.id), persona] as const)
      .filter(([id, persona]) => !!id && !persona.isBuiltInUser),
  );
  const apiConfigs = new Map(
    (input.apiConfigs || [])
      .map((api) => [trimText(api.id), api] as const)
      .filter(([id, api]) => !!id && !!api.enableText),
  );
  const options: DepartmentPersonaOption[] = [];
  for (const department of input.departments || []) {
    const departmentId = trimText(department.id);
    if (!departmentId) continue;
    const apiConfigId = departmentConversationApiConfigId(department, input);
    const apiConfig = apiConfigId ? apiConfigs.get(apiConfigId) : null;
    if (!apiConfig) continue;
    const departmentName = trimText(department.name) || departmentId;
    const agentIds = Array.from(new Set((department.agentIds || []).map(trimText).filter(Boolean)));
    for (const agentId of agentIds) {
      const persona = personas.get(agentId);
      if (!persona) continue;
      const agentName = trimText(persona.name) || agentId;
      options.push({
        id: departmentPersonaOptionId(departmentId, agentId),
        departmentId,
        agentId,
        departmentName,
        agentName,
        label: `${departmentName} / ${agentName}`,
        name: departmentName,
        ownerAgentId: agentId,
        ownerName: agentName,
        providerName: trimText(apiConfig.name || apiConfig.id) || undefined,
        modelName: trimText(apiConfig.model) || undefined,
        apiConfigId,
        childDepartmentIds: Array.isArray(department.childDepartmentIds)
          ? department.childDepartmentIds.map(trimText).filter(Boolean)
          : [],
      });
    }
  }
  return options;
}

export function findDepartmentPersonaOption(
  options: DepartmentPersonaOption[],
  departmentId?: string | null,
  agentId?: string | null,
): DepartmentPersonaOption | null {
  const did = trimText(departmentId);
  const aid = trimText(agentId);
  if (!did) return null;
  if (aid) {
    return options.find((option) => option.departmentId === did && option.agentId === aid) || null;
  }
  return options.find((option) => option.departmentId === did) || null;
}
