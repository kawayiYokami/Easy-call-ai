export type FixedIntervalUnit = "minutes" | "hours" | "days";

export type FixedIntervalFromCron = {
  repeatEvery: number;
  repeatUnit: FixedIntervalUnit;
};

function parseSortedUniqueNumericList(
  value: string,
  min: number,
  max: number,
): number[] | null {
  const normalized = String(value || "").trim();
  if (!normalized) return null;
  if (/^\d{1,2}$/.test(normalized)) {
    const single = Number.parseInt(normalized, 10);
    return Number.isInteger(single) && single >= min && single <= max ? [single] : null;
  }
  if (!/^\d{1,2}(,\d{1,2})+$/.test(normalized)) {
    return null;
  }
  const values = normalized
    .split(",")
    .map((item) => Number.parseInt(item, 10))
    .filter((item) => Number.isInteger(item) && item >= min && item <= max)
    .sort((left, right) => left - right);
  if (!values.length) return null;
  const uniqueValues = Array.from(new Set(values));
  return uniqueValues.length === values.length ? uniqueValues : null;
}

function equalStep(values: number[], cycle: number): number | null {
  if (!values.length) return null;
  if (values.length === 1) return cycle;
  const diffs = values.map((value, index) => {
    const next = index === values.length - 1 ? values[0] + cycle : values[index + 1];
    return next - value;
  });
  const firstDiff = diffs[0];
  return firstDiff > 0 && diffs.every((item) => item === firstDiff) ? firstDiff : null;
}

export function inferFixedIntervalFromCronExpression(value: string): FixedIntervalFromCron | null {
  const parts = String(value || "").trim().split(/\s+/);
  if (parts.length !== 5) return null;

  const [minutePart, hourPart, dayOfMonthPart, monthPart, dayOfWeekPart] = parts;
  if (dayOfMonthPart !== "*" || monthPart !== "*" || dayOfWeekPart !== "*") {
    return null;
  }

  if (minutePart === "*" && hourPart === "*") {
    return { repeatEvery: 1, repeatUnit: "minutes" };
  }

  const minutes = parseSortedUniqueNumericList(minutePart, 0, 59);
  if (!minutes) return null;

  if (hourPart === "*") {
    const minuteStep = equalStep(minutes, 60);
    if (!minuteStep) return null;
    if (minuteStep === 60) {
      return { repeatEvery: 1, repeatUnit: "hours" };
    }
    return { repeatEvery: minuteStep, repeatUnit: "minutes" };
  }

  const hours = parseSortedUniqueNumericList(hourPart, 0, 23);
  if (!hours || minutes.length !== 1) return null;
  const hourStep = equalStep(hours, 24);
  if (!hourStep) return null;
  if (hourStep === 24) {
    return { repeatEvery: 1, repeatUnit: "days" };
  }
  return { repeatEvery: hourStep, repeatUnit: "hours" };
}

export function inferMonthlyIntervalFromCronExpression(value: string): number | null {
  const parts = String(value || "").trim().split(/\s+/);
  if (parts.length !== 5) return null;

  const [minutePart, hourPart, dayOfMonthPart, monthPart, dayOfWeekPart] = parts;
  if (dayOfWeekPart !== "*") return null;

  const minute = Number.parseInt(minutePart, 10);
  const hour = Number.parseInt(hourPart, 10);
  const dayOfMonth = Number.parseInt(dayOfMonthPart, 10);
  if (
    !/^\d{1,2}$/.test(minutePart)
    || !Number.isInteger(minute)
    || minute < 0
    || minute > 59
    || !/^\d{1,2}$/.test(hourPart)
    || !Number.isInteger(hour)
    || hour < 0
    || hour > 23
    || !/^\d{1,2}$/.test(dayOfMonthPart)
    || !Number.isInteger(dayOfMonth)
    || dayOfMonth < 1
    || dayOfMonth > 31
  ) {
    return null;
  }

  if (monthPart === "*") return 1;
  if (!/^\d{1,2}(,\d{1,2})+$/.test(monthPart)) return null;

  const months = monthPart
    .split(",")
    .map((item) => Number.parseInt(item, 10))
    .filter((item) => Number.isInteger(item) && item >= 1 && item <= 12)
    .sort((left, right) => left - right);
  if (months.length < 2) return null;

  const uniqueMonths = Array.from(new Set(months));
  if (uniqueMonths.length !== months.length) return null;
  if (uniqueMonths.length === 1) return 12;

  const diffs = uniqueMonths.map((month, index) => {
    const nextMonth = index === uniqueMonths.length - 1 ? uniqueMonths[0] + 12 : uniqueMonths[index + 1];
    return nextMonth - month;
  });
  const firstDiff = diffs[0];
  return firstDiff > 0 && diffs.every((item) => item === firstDiff) ? firstDiff : null;
}
