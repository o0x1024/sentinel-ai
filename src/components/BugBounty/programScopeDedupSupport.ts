export type ProgramScopeIdentityInput = {
  scope_type?: string | null
  target_type?: string | null
  target?: string | null
}

export function normalizeProgramScopeIdentityTarget(value: string | null | undefined): string {
  return String(value || '').trim().toLowerCase()
}

export function parseProgramScopeTargetLines(value: string | null | undefined): string[] {
  const seenTargets = new Set<string>()
  const targets: string[] = []

  for (const line of String(value || '').split(/\r?\n/)) {
    const target = line.trim()
    const normalizedTarget = normalizeProgramScopeIdentityTarget(target)
    if (!normalizedTarget || seenTargets.has(normalizedTarget)) {
      continue
    }

    seenTargets.add(normalizedTarget)
    targets.push(target)
  }

  return targets
}

export function findDuplicateProgramScopeTargets(
  scopes: ProgramScopeIdentityInput[],
  candidate: ProgramScopeIdentityInput,
  targets: string[],
): string[] {
  const candidateScopeType = String(candidate.scope_type || '').trim()
  const candidateTargetType = String(candidate.target_type || '').trim()

  if (!candidateScopeType || !candidateTargetType || targets.length === 0) {
    return []
  }

  return targets.filter(target =>
    hasDuplicateProgramScopeTarget(scopes, {
      ...candidate,
      target,
    }),
  )
}

export function hasDuplicateProgramScopeTarget(
  scopes: ProgramScopeIdentityInput[],
  candidate: ProgramScopeIdentityInput,
): boolean {
  const candidateScopeType = String(candidate.scope_type || '').trim()
  const candidateTargetType = String(candidate.target_type || '').trim()
  const candidateTarget = normalizeProgramScopeIdentityTarget(candidate.target)

  if (!candidateScopeType || !candidateTargetType || !candidateTarget) {
    return false
  }

  return scopes.some(scope =>
    String(scope.scope_type || '').trim() === candidateScopeType
    && String(scope.target_type || '').trim() === candidateTargetType
    && normalizeProgramScopeIdentityTarget(scope.target) === candidateTarget,
  )
}
