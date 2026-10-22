export interface HunspellModule {
  readonly HEAPU8: Uint8Array
  readonly HEAPU32: Uint32Array
  readonly FS: {
    mkdir(path: string): void
    writeFile(path: string, data: Uint8Array): void
    unlink(path: string): void
  }
  UTF8ToString(pointer: number): string
  stringToNewUTF8(value: string): number
  _malloc(bytes: number): number
  _free(pointer: number): void
  _Hunspell_create(affPath: number, dicPath: number): number
  _Hunspell_destroy(handle: number): void
  _Hunspell_spell(handle: number, word: number): number
  _Hunspell_suggest(handle: number, list: number, word: number): number
  _Hunspell_free_list(handle: number, list: number, count: number): void
  _Hunspell_add(handle: number, word: number): number
  _Hunspell_remove(handle: number, word: number): number
  _Hunspell_get_dic_encoding(handle: number): number
}

export interface HunspellModuleOptions {
  locateFile?: (path: string, scriptDirectory: string) => string
  wasmBinary?: ArrayBuffer | Uint8Array
}

export default function createHunspellModule(options?: HunspellModuleOptions): Promise<HunspellModule>
