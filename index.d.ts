export type DictionaryId =
  | 'en-US'
  | 'en-GB'
  | 'de-DE'
  | 'ru-RU'
  | 'uk-UA'
  | 'fr-FR'
  | 'es-ES'
  | 'it-IT'
  | 'pt-BR'
  | 'pt-PT'
  | 'nl-NL'
  | 'pl-PL'

export interface Dictionary {
  readonly id: DictionaryId
  /** The npm release the files were taken from, e.g. `dictionary-en@4.0.0`. */
  readonly upstream: string
  /** The publisher's license declaration, verbatim. */
  readonly license: string
}

export declare const dictionaries: readonly Dictionary[]
export declare const dictionaryIds: readonly DictionaryId[]
