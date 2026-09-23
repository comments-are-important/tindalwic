
import { InputStream } from "@lezer/lr"

export const EOF = -1 // this one isn't even a char
export const TAB = 9 // officially HT in ASCII
export const LF = 10
export const BANG = 33 /* ! */ // not reserved
export const HASH = 35 /* # */
export const SLASH = 47 /* / */
export const EQ = 61 /* = */
export const AT = 64 /* @ */
export const BRA_A = 60  /* < */, A_KET = 62 /* > */
export const BRA_S = 91  /* [ */, S_KET = 93 /* ] */
export const BRA_C = 123 /* { */, C_KET = 125 /* } */

export function reserved(char: number): boolean {
    switch (char) {
        case EOF: case TAB: case LF: case HASH: case SLASH: case EQ: case AT:
        case BRA_A: case A_KET: case BRA_S: case S_KET: case BRA_C: case C_KET:
            return true
    }
    return false
}

export function show(char: number | InputStream): string {
    if (char instanceof InputStream) char = char.next
    switch (char) {
        case EOF: return "EOF"
        case 0: return "NUL"
        case 1: return "SOH"
        case 2: return "STX"
        case 3: return "ETX"
        case 4: return "EOT"
        case 5: return "ENQ"
        case 6: return "ACK"
        case 7: return "BEL"
        case 8: return "BS"
        case TAB: return "TAB"
        case LF: return "LF"
        case 11: return "VT"
        case 12: return "FF"
        case 13: return "CR"
        case 14: return "SO"
        case 15: return "SI"
        case 16: return "DLE"
        case 17: return "DC1"
        case 18: return "DC2"
        case 19: return "DC3"
        case 20: return "DC4"
        case 21: return "NAK"
        case 22: return "SYN"
        case 23: return "ETB"
        case 24: return "CAN"
        case 25: return "EM"
        case 26: return "SUB"
        case 27: return "ESC"
        case 28: return "FS"
        case 29: return "GS"
        case 30: return "RS"
        case 31: return "US"
        case 32: return "SPACE"
        case 127: return "DEL"
    }
    if (0 <= char && char <= 0x10FFF) return String.fromCodePoint(char)
    return `0x${char.toString(16).toUpperCase()}`
}
