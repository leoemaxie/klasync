/** Shared group colours so the 2D and 3D maps (and the legend) agree. */
export const GROUP_COLORS = [
  '#dc5000',
  '#e0a458',
  '#7fb069',
  '#6aa6c9',
  '#b48ead',
  '#d98880',
  '#9aa5b1',
] as const;

export const groupsOf = (nodes: { group: string }[]) => [
  ...new Set(nodes.map((n) => n.group)),
];

export const colorForGroup = (groups: string[], group: string) =>
  GROUP_COLORS[Math.max(groups.indexOf(group), 0) % GROUP_COLORS.length];
