import type { ReactNode } from "react";
export function TerminalPanel({
  title,
  span = 12,
  children,
  className = "",
}: {
  title: string;
  span?: number;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={`card col-${span} ${className}`}>
      <h2>{title}</h2>
      {children}
    </section>
  );
}
export function MetricReadout({
  label,
  value,
  unit = "",
}: {
  label: string;
  value: ReactNode;
  unit?: string;
}) {
  return (
    <div className="metric">
      <small>{label}</small>
      <strong className="crt-readout">{value}</strong>
      <small>{unit}</small>
    </div>
  );
}
export function CRTText({ children }: { children: ReactNode }) {
  return <span className="crt-text">{children}</span>;
}
