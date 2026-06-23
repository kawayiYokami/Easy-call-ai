import { describe, expect, it } from "vitest";
import { taskEditorFormFromEntry, type TaskEntry } from "./task-editor";

function buildTaskEntry(cronExpression: string): TaskEntry {
  return {
    taskId: "task-1",
    conversationId: "conversation-1",
    orderIndex: 1,
    goal: "向用户报时",
    why: "",
    todo: "直接向用户报告当前时间",
    completionState: "active",
    completionConclusion: "",
    progressNotes: [],
    trigger: {
      run_at: "2026-06-24T01:24:00+08:00",
      cron_expression: cronExpression,
    },
    createdAtLocal: "2026-06-24T01:19:00+08:00",
    updatedAtLocal: "2026-06-24T01:19:00+08:00",
  };
}

describe("taskEditorFormFromEntry", () => {
  it("maps stepped minute cron back to the interval editor", () => {
    const form = taskEditorFormFromEntry(buildTaskEntry("*/2 * * * *"));
    expect(form.scheduleMode).toBe("interval");
    expect(form.repeatWeeks).toBe("0");
    expect(form.repeatDays).toBe("0");
    expect(form.repeatHours).toBe("0");
    expect(form.preservedCronExpression).toBe("*/2 * * * *");
  });
});
