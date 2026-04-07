import { describe, expect, it } from 'vitest'
import {
  decodeIntruderStructuredDictionarySources,
  encodeIntruderStructuredDictionarySources,
} from './intruderDictionarySources'

describe('intruderDictionarySources', () => {
  it('encodes string references into structured dictionary sources', () => {
    const encoded = encodeIntruderStructuredDictionarySources(
      ['DICT-1', 'default:username'],
      {
        'DICT-1': {
          id: 'DICT-1',
          name: 'Common Users',
          dict_type: 'username',
        },
      },
    )

    expect(encoded).toEqual([
      {
        type: 'dictionary',
        dictionaryId: 'DICT-1',
        dictionaryName: 'Common Users',
      },
      {
        type: 'default_dictionary',
        dictType: 'username',
      },
    ])
  })

  it('decodes structured dictionary sources back to UI reference strings', () => {
    const decoded = decodeIntruderStructuredDictionarySources([
      {
        type: 'dictionary',
        dictionaryId: 'DICT-1',
        dictionaryName: 'Common Users',
      },
      {
        type: 'default_dictionary',
        dictType: 'username',
      },
    ])

    expect(decoded).toEqual(['DICT-1', 'default:username'])
  })
})
