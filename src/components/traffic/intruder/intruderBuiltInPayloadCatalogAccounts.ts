import { deduplicatePayloadItems } from './intruderBuiltInPayloadCatalogShared'
import type {
  IntruderBuiltInPayloadListDefinition,
  IntruderPayloadTemplateDefinition,
} from './intruderBuiltInPayloadCatalogShared'

interface ChineseNameSeed {
  chinese: string
  familyPinyin: string
  givenPinyin: string
}

const CHINESE_NAME_SEEDS: ChineseNameSeed[] = [
  { chinese: '张三', familyPinyin: 'zhang', givenPinyin: 'san' },
  { chinese: '李四', familyPinyin: 'li', givenPinyin: 'si' },
  { chinese: '王五', familyPinyin: 'wang', givenPinyin: 'wu' },
  { chinese: '赵六', familyPinyin: 'zhao', givenPinyin: 'liu' },
  { chinese: '小明', familyPinyin: 'xiao', givenPinyin: 'ming' },
  { chinese: '小红', familyPinyin: 'xiao', givenPinyin: 'hong' },
  { chinese: '王磊', familyPinyin: 'wang', givenPinyin: 'lei' },
  { chinese: '李娜', familyPinyin: 'li', givenPinyin: 'na' },
  { chinese: '陈晨', familyPinyin: 'chen', givenPinyin: 'chen' },
  { chinese: '刘洋', familyPinyin: 'liu', givenPinyin: 'yang' },
]

function buildChinesePinyinUsernames(): string[] {
  return CHINESE_NAME_SEEDS.map((seed) => `${seed.familyPinyin}${seed.givenPinyin}`)
}

function buildChineseInitialUsernames(): string[] {
  return CHINESE_NAME_SEEDS.map((seed) => `${seed.familyPinyin[0]}${seed.givenPinyin[0]}`)
}

function buildChineseDerivedUsernames(): string[] {
  return deduplicatePayloadItems(
    CHINESE_NAME_SEEDS.flatMap((seed) => {
      const family = seed.familyPinyin
      const given = seed.givenPinyin
      const f = family[0] || ''
      const g = given[0] || ''

      return [
        `${family}${given}`,
        `${family}.${given}`,
        `${family}_${given}`,
        `${family}-${given}`,
        `${f}${given}`,
        `${family}${g}`,
        `${given}${family}`,
        `${f}${g}`,
      ]
    }),
  )
}

const COMMON_USERNAMES = ['admin', 'administrator', 'root', 'test', 'guest', 'demo', 'user', 'dev', 'support']
const COMMON_EMAILS = [
  'admin@example.com',
  'test@example.com',
  'guest@example.com',
  'dev@example.com',
  'support@example.com',
  'security@example.com',
  'hr@example.com',
  'finance@example.com',
  'it@example.com',
  'service@example.com',
]
const CHINESE_MOBILE_NUMBERS = [
  '13800138000',
  '13900139000',
  '13600000000',
  '13712345678',
  '15012345678',
  '15888888888',
  '18612345678',
  '18800000000',
  '19912345678',
]
const COMMON_PASSWORDS = ['admin', 'password', '123456', '12345678', 'qwerty', 'welcome', 'letmein', 'secret']
const WEAK_PASSWORD_PATTERNS = [
  'Admin@123',
  'Password@123',
  'P@ssw0rd',
  'Passw0rd!',
  'Welcome123',
  'Test@123',
  'Qwe123!@#',
  'Aa123456',
  '123qweASD',
  'Company@123',
]
const COMMON_PINS = ['0000', '1111', '1234', '1122', '1212', '1314', '2580', '6666', '8888', '9999']

export const INTRUDER_ACCOUNT_PAYLOAD_LISTS: IntruderBuiltInPayloadListDefinition[] = [
  {
    id: 'usernames',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.usernames',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.usernames',
    aliases: ['user', 'username', 'login', 'account', '用户名', '账号'],
    tags: ['account', 'username'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['user', 'username', 'login', 'account', 'member'],
    items: COMMON_USERNAMES,
  },
  {
    id: 'usernames-chinese',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.chineseUsernames',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.chineseUsernames',
    aliases: ['chinese', 'username', 'cn', '中文用户名', '姓名'],
    tags: ['account', 'username', 'chinese'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['user', 'username', 'name', 'realname', '姓名'],
    items: CHINESE_NAME_SEEDS.map((seed) => seed.chinese),
  },
  {
    id: 'usernames-chinese-pinyin',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.chineseUsernamesPinyin',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.chineseUsernamesPinyin',
    aliases: ['pinyin', 'romanized', '用户名拼音', '拼音'],
    tags: ['account', 'username', 'chinese', 'pinyin'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['user', 'username', 'login', 'account'],
    items: buildChinesePinyinUsernames(),
  },
  {
    id: 'usernames-chinese-initials',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.chineseUsernamesInitials',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.chineseUsernamesInitials',
    aliases: ['initials', 'short username', '首字母', '简写'],
    tags: ['account', 'username', 'chinese', 'initials'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['user', 'username', 'login', 'account'],
    items: buildChineseInitialUsernames(),
  },
  {
    id: 'usernames-chinese-derived',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.chineseUsernamesDerived',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.chineseUsernamesDerived',
    aliases: ['derived', 'formats', 'username formats', '格式', '派生'],
    tags: ['account', 'username', 'chinese', 'derived'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['user', 'username', 'login', 'account'],
    items: buildChineseDerivedUsernames(),
  },
  {
    id: 'emails-common',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.commonEmails',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.commonEmails',
    aliases: ['email', 'mail', '邮箱', '邮件'],
    tags: ['account', 'email'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['email', 'mail'],
    items: COMMON_EMAILS,
  },
  {
    id: 'phones-cn',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.chineseMobileNumbers',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.chineseMobileNumbers',
    aliases: ['phone', 'mobile', 'tel', '手机号', '手机', '电话'],
    tags: ['account', 'phone'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['phone', 'mobile', 'tel', 'msisdn'],
    items: CHINESE_MOBILE_NUMBERS,
  },
  {
    id: 'passwords',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.passwords',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.passwords',
    aliases: ['password', 'passwd', 'pwd', '密码'],
    tags: ['account', 'password'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['password', 'passwd', 'pwd', 'pass'],
    items: COMMON_PASSWORDS,
  },
  {
    id: 'passwords-patterns',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.weakPasswordsPatterns',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.weakPasswordsPatterns',
    aliases: ['password', 'pattern', 'weak password', '弱密码', '模式'],
    tags: ['account', 'password', 'weak'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['password', 'passwd', 'pwd', 'pass'],
    items: WEAK_PASSWORD_PATTERNS,
  },
  {
    id: 'pins-common',
    categoryKey: 'trafficAnalysis.intruder.builtInPayloadListCategories.accounts',
    labelKey: 'trafficAnalysis.intruder.builtInPayloadLists.commonPins',
    descriptionKey: 'trafficAnalysis.intruder.builtInPayloadDescriptions.commonPins',
    aliases: ['pin', 'otp', 'code', '验证码', 'pin码'],
    tags: ['account', 'pin'],
    recommendedAttackTypes: ['sniper', 'pitchfork'],
    recommendedPositionHints: ['pin', 'otp', 'code', 'verify'],
    items: COMMON_PINS,
  },
]

export const INTRUDER_PAYLOAD_TEMPLATES: IntruderPayloadTemplateDefinition[] = [
  {
    id: 'credentials-common',
    labelKey: 'trafficAnalysis.intruder.payloadTemplates.credentialsCommon',
    descriptionKey: 'trafficAnalysis.intruder.payloadTemplateDescriptions.credentialsCommon',
    tags: ['login', 'auth', 'username', 'password'],
    recommendedAttackType: 'pitchfork',
    recommendedPositionHints: ['user', 'username', 'login', 'password', 'passwd', 'pwd'],
    sets: [
      {
        nameKey: 'trafficAnalysis.intruder.payloadTemplateSetNames.usernames',
        sourceId: 'usernames',
      },
      {
        nameKey: 'trafficAnalysis.intruder.payloadTemplateSetNames.passwords',
        sourceId: 'passwords-patterns',
      },
    ],
  },
  {
    id: 'credentials-chinese',
    labelKey: 'trafficAnalysis.intruder.payloadTemplates.credentialsChinese',
    descriptionKey: 'trafficAnalysis.intruder.payloadTemplateDescriptions.credentialsChinese',
    tags: ['login', 'auth', 'chinese', 'username', 'password'],
    recommendedAttackType: 'pitchfork',
    recommendedPositionHints: ['user', 'username', 'login', 'password', 'passwd', 'pwd'],
    sets: [
      {
        nameKey: 'trafficAnalysis.intruder.payloadTemplateSetNames.chineseUsernames',
        sourceId: 'usernames-chinese-derived',
      },
      {
        nameKey: 'trafficAnalysis.intruder.payloadTemplateSetNames.passwords',
        sourceId: 'passwords-patterns',
      },
    ],
  },
  {
    id: 'phone-pin',
    labelKey: 'trafficAnalysis.intruder.payloadTemplates.phonePin',
    descriptionKey: 'trafficAnalysis.intruder.payloadTemplateDescriptions.phonePin',
    tags: ['phone', 'mobile', 'pin', 'otp'],
    recommendedAttackType: 'pitchfork',
    recommendedPositionHints: ['phone', 'mobile', 'pin', 'otp', 'code'],
    sets: [
      {
        nameKey: 'trafficAnalysis.intruder.payloadTemplateSetNames.phoneNumbers',
        sourceId: 'phones-cn',
      },
      {
        nameKey: 'trafficAnalysis.intruder.payloadTemplateSetNames.pins',
        sourceId: 'pins-common',
      },
    ],
  },
]
