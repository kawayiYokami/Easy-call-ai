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
