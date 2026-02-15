import type { MemberInfo } from "@/types/api";

export const AGENT_COLORS = ['#f59e0b', '#a78bfa', '#38bdf8', '#fb7185', '#34d399', '#f472b6', '#60a5fa', '#fbbf24'];

export function agentColor(index: number) {
  return AGENT_COLORS[index % AGENT_COLORS.length]!;
}

export function agentInitials(name: string) {
  if (name === 'lead' || name === 'team-lead') return 'TL';
  const parts = name.split('-');
  if (parts.length > 1 && /^\d+$/.test(parts[parts.length - 1]!)) {
    return parts[0]![0]!.toUpperCase() + parts[parts.length - 1]!;
  }
  return name.slice(0, 2).toUpperCase();
}

export function agentColorIndex(name: string, members: MemberInfo[]) {
  if (name === 'lead' || name === 'team-lead') return 0;
  const idx = members.findIndex(m => m.name === name);
  return idx >= 0 ? idx + 1 : 1;
}

export function fmtCost(usd: number) {
  return usd >= 1 ? `$${usd.toFixed(2)}` : `$${usd.toFixed(4)}`;
}

export function fmtTokens(n: number) {
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}k`;
  return `${n}`;
}

export function shortModel(model: string) {
  if (model.includes('sonnet')) return 'sonnet';
  if (model.includes('haiku')) return 'haiku';
  if (model.includes('opus')) return 'opus';
  return model.split('-')[0] || model;
}
