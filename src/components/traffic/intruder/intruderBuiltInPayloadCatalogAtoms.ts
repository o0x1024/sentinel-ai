import { buildRange } from './intruderBuiltInPayloadCatalogShared'
import type { IntruderBuiltInPayloadListDefinition } from './intruderBuiltInPayloadCatalogShared'

const LOWERCASE_LETTERS = buildRange(97, 122)
const UPPERCASE_LETTERS = buildRange(65, 90)
const DIGITS = Array.from({ length: 10 }, (_, index) => String(index))
const HEX_LOWERCASE = [...DIGITS, ...buildRange(97, 102)]

export const INTRUDER_ATOM_PAYLOAD_LISTS: IntruderBuiltInPayloadListDefinition[] = [
  {
    id: 'short-words',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.atoms',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.shortWords',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.shortWords',
    aliases: ['short', 'word', 'token', '词', '短词'],
    tags: ['atom', 'short', 'generic'],
    recommendedAttackTypes: ['sniper', 'batteringRam'],
    recommendedPositionHints: ['token', 'code', 'debug', 'stage', 'env'],
    items: ['test', 'dev', 'tmp', 'demo', 'api', 'prod', 'stage', 'debug', 'backup'],
  },
  {
    id: 'a-z',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.atoms',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.aToZLower',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.aToZLower',
    aliases: ['letters', 'alphabet', 'lowercase', '字母', '小写'],
    tags: ['atom', 'alphabet', 'lowercase'],
    recommendedAttackTypes: ['sniper', 'batteringRam'],
    recommendedPositionHints: ['char', 'letter', 'column'],
    items: LOWERCASE_LETTERS,
  },
  {
    id: 'A-Z',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.atoms',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.aToZUpper',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.aToZUpper',
    aliases: ['letters', 'alphabet', 'uppercase', '字母', '大写'],
    tags: ['atom', 'alphabet', 'uppercase'],
    recommendedAttackTypes: ['sniper', 'batteringRam'],
    recommendedPositionHints: ['char', 'letter', 'column'],
    items: UPPERCASE_LETTERS,
  },
  {
    id: '0-9',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.atoms',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.zeroToNine',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.zeroToNine',
    aliases: ['digits', 'numbers', 'numeric', '数字'],
    tags: ['atom', 'numeric'],
    recommendedAttackTypes: ['sniper', 'batteringRam'],
    recommendedPositionHints: ['id', 'pin', 'otp', 'code', 'number'],
    items: DIGITS,
  },
  {
    id: 'hex-lower',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.atoms',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.hexLower',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.hexLower',
    aliases: ['hex', 'hexadecimal', '小写hex', '16进制'],
    tags: ['atom', 'hex'],
    recommendedAttackTypes: ['sniper', 'batteringRam'],
    recommendedPositionHints: ['hash', 'hex', 'token', 'digest'],
    items: HEX_LOWERCASE,
  },
]
