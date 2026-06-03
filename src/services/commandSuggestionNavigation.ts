export function getNextCommandSuggestionIndex(
  currentIndex: number,
  suggestionCount: number,
  direction: 1 | -1,
) {
  if (suggestionCount <= 0) {
    return -1
  }

  if (currentIndex < 0) {
    return direction === 1 ? 0 : suggestionCount - 1
  }

  return (currentIndex + direction + suggestionCount) % suggestionCount
}
