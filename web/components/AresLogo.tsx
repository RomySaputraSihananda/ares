export default function AresLogo({ size = 28 }: { size?: number }) {
  const id = `ares-logo-${size}`;
  return (
    <svg width={size} height={size} viewBox="120 70 260 270" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
      <defs>
        <linearGradient id={`${id}-l`} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#5e6ad2" />
          <stop offset="100%" stopColor="#3d4aaa" />
        </linearGradient>
        <linearGradient id={`${id}-r`} x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#828fff" />
          <stop offset="100%" stopColor="#5e6ad2" />
        </linearGradient>
        <linearGradient id={`${id}-c`} x1="0%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stopColor="#c4c8f8" />
          <stop offset="100%" stopColor="#828fff" />
        </linearGradient>
      </defs>
      <g transform="translate(100, 80)">
        <path d="M 150 0 L 30 240 L 90 320 L 150 140 Z"           fill={`url(#${id}-l)`} />
        <path d="M 150 0 L 270 240 L 210 320 L 150 140 Z"          fill={`url(#${id}-r)`} />
        <path d="M 150 40 L 110 140 L 150 120 L 190 140 Z"         fill={`url(#${id}-c)`} />
        <path d="M 95 210 L 205 210 L 150 140 Z" fill="#000" opacity="0.25" />
      </g>
    </svg>
  );
}
