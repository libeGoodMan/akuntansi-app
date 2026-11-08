import * as React from "react";
import { Input } from "@/components/ui/input";
import { formatMoney, parseMoney } from "@/lib/money";
import { cn } from "@/lib/utils";

interface Props extends Omit<React.ComponentProps<"input">, "value" | "onChange"> {
  value: string;
  onValueChange: (v: string) => void;
  allowNegative?: boolean;
}

/** Input nominal rupiah: rata kanan, diformat "1.250.000,50" saat fokus pergi. */
export function MoneyInput({
  value,
  onValueChange,
  allowNegative,
  className,
  onBlur,
  ...rest
}: Props) {
  const parsed = parseMoney(value, { allowNegative });
  const invalid = value.trim() !== "" && parsed === null;

  return (
    <Input
      inputMode="decimal"
      autoComplete="off"
      placeholder="0,00"
      aria-invalid={invalid || undefined}
      className={cn("text-right tabular-nums", className)}
      value={value}
      onFocus={(e) => e.currentTarget.select()}
      onChange={(e) => onValueChange(e.target.value)}
      onBlur={(e) => {
        if (parsed !== null) onValueChange(formatMoney(parsed));
        onBlur?.(e);
      }}
      {...rest}
    />
  );
}
