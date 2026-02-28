export function useClipboard() {
  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
  }

  return { copyToClipboard };
}
