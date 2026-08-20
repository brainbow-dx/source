// This exact return value is a magic sentinel `QUIT_SENTINEL` (main.rs) watches for to actually
// exit — don't "clean this up" into a friendlier string, that'll just silently break /quit.
export const run = async () => "💀";
