import { describe, expect, it } from "vitest";
import { formatCompact, formatMoney, formatMoneyOrBlank, parseMoney } from "./money";

describe("formatMoney", () => {
  it("memformat sen ke format Indonesia", () => {
    expect(formatMoney(0)).toBe("0,00");
    expect(formatMoney(5)).toBe("0,05");
    expect(formatMoney(100)).toBe("1,00");
    expect(formatMoney(125000050)).toBe("1.250.000,50");
    expect(formatMoney(100000000000)).toBe("1.000.000.000,00");
  });
  it("menangani negatif", () => {
    expect(formatMoney(-125000050)).toBe("-1.250.000,50");
    expect(formatMoney(-5)).toBe("-0,05");
  });
  it("0 kosong untuk sel debit/kredit", () => {
    expect(formatMoneyOrBlank(0)).toBe("");
    expect(formatMoneyOrBlank(1000)).toBe("10,00");
  });
});

describe("parseMoney", () => {
  it("membaca format Indonesia", () => {
    expect(parseMoney("1.250.000,50")).toBe(125000050);
    expect(parseMoney("1250000")).toBe(125000000);
    expect(parseMoney("0,5")).toBe(50);
    expect(parseMoney("0,05")).toBe(5);
    expect(parseMoney("Rp 10.000")).toBe(1000000);
    expect(parseMoney("  42  ")).toBe(4200);
  });
  it("kosong atau tidak valid menjadi null", () => {
    expect(parseMoney("")).toBeNull();
    expect(parseMoney("   ")).toBeNull();
    expect(parseMoney("abc")).toBeNull();
    expect(parseMoney("1,234")).toBeNull(); // 3 desimal
    expect(parseMoney("1,2,3")).toBeNull();
    expect(parseMoney("12.34")).toBeNull(); // titik bukan desimal
    expect(parseMoney("1.2345")).toBeNull();
  });
  it("negatif hanya bila diizinkan", () => {
    expect(parseMoney("-5")).toBeNull();
    expect(parseMoney("-5", { allowNegative: true })).toBe(-500);
  });
  it("tidak kehilangan ketelitian (bolak-balik)", () => {
    for (const cents of [1, 99, 100, 101, 123456789, 99999999999]) {
      expect(parseMoney(formatMoney(cents))).toBe(cents);
    }
  });
  it("menolak angka di luar rentang aman", () => {
    expect(parseMoney("99.999.999.999.999.999")).toBeNull();
  });
});

describe("formatCompact", () => {
  it("meringkas ke rb, jt, M, T", () => {
    expect(formatCompact(0)).toBe("0");
    expect(formatCompact(45000)).toBe("450");
    expect(formatCompact(450000)).toBe("4,5 rb");
    expect(formatCompact(1250000000)).toBe("12,5 jt");
    expect(formatCompact(21800000000)).toBe("218 jt");
    expect(formatCompact(125000000000)).toBe("1,25 M");
    expect(formatCompact(100000000000000)).toBe("1 T");
  });
  it("menangani negatif", () => {
    expect(formatCompact(-1250000000)).toBe("-12,5 jt");
  });
});
