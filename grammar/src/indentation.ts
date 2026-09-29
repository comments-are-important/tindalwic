
export interface Indentation {
    readonly parent: Indentation | null
    readonly hash: number
    // above are normal fields
    // below come from prototype hacking
    readonly depth: number
    readonly sealed: boolean
    seal(): Indentation
}

export const prototypes: Array<object> = []

function hashAfterInstallingPrototype(context: Indentation): number {
    const parent = context.parent
    if (parent === null) return 0
    if (parent.depth === Number.MAX_SAFE_INTEGER)
        // very unlikely to get here because memory would have been filled up
        throw new RangeError("can't go deeper than MAX_SAFE_INTEGER")
    const length = 3 * (parent.depth + 1)
    while (prototypes.length < length) {
        const depth = (1 + Math.floor(prototypes.length / 3)) | 0
        const meta = { depth, sealed: false }
        prototypes.push(Object.setPrototypeOf({ ...meta }, Text.prototype))
        prototypes.push(Object.setPrototypeOf({ ...meta }, Dict.prototype))
        prototypes.push(Object.setPrototypeOf(meta, List.prototype))
    }
    switch (Object.getPrototypeOf(context)) {
        case Text.prototype:
            Object.setPrototypeOf(context, prototypes[length - 3])
            break
        case Dict.prototype:
            Object.setPrototypeOf(context, prototypes[length - 2])
            break
        case List.prototype:
            Object.setPrototypeOf(context, prototypes[length - 1])
            break
    }
    let hash = 17 // emulates java.util.Objects.hash()
    hash = (31 * hash + parent.hash) | 0
    hash = (31 * hash + context.depth) | 0
    return hash
}

function sealPrototype(context: Indentation): Indentation {
    if (context.sealed) return context
    const proto = Object.create(Object.getPrototypeOf(context))
    proto.sealed = true
    const clone = Object.assign(Object.create(proto), context)
    clone.hash = (31 * context.hash + 127) | 0 // emulates annotation (kinda)
    return clone
}

export class File implements Indentation {
    readonly parent = null
    readonly hash = 0
    readonly depth = 0
    declare sealed: boolean
    constructor() {
        const meta = { depth: 0, sealed: false }
        const proto = Object.setPrototypeOf(meta, File.prototype)
        Object.setPrototypeOf(this, proto)
    }
    toString(): string {
        return `${this.sealed ? "s" : ""}F`
    }
    seal(): Indentation {
        return sealPrototype(this)
    }
}

export class Text implements Indentation {
    readonly parent: Indentation | null
    readonly hash: number
    declare depth: number
    declare sealed: boolean
    constructor(parent: File | Dict) {
        this.parent = parent
        this.hash = hashAfterInstallingPrototype(this)
    }
    toString(): string {
        return `${this.sealed ? "s" : ""}T${this.depth}`
    }
    seal(): Indentation {
        return sealPrototype(this)
    }
}

export class Dict implements Indentation {
    readonly parent: Indentation | null
    readonly hash: number
    declare depth: number
    declare sealed: boolean
    constructor(parent: File | Dict) {
        this.parent = parent
        let hash = hashAfterInstallingPrototype(this)
        hash = (31 * hash + 1231) | 0 // emulates a true
        this.hash = hash
    }
    toString(): string {
        return `${this.sealed ? "s" : ""}D${this.depth}`
    }
    seal(): Indentation {
        return sealPrototype(this)
    }
}

export class List implements Indentation {
    readonly parent: Indentation | null
    readonly hash: number
    declare depth: number
    declare sealed: boolean
    constructor(parent: File | Dict) {
        this.parent = parent
        let hash = hashAfterInstallingPrototype(this)
        hash = (31 * hash + 1237) | 0 // emulates a false
        this.hash = hash
    }
    toString(): string {
        return `${this.sealed ? "s" : ""}L${this.depth}`
    }
    seal(): Indentation {
        return sealPrototype(this)
    }
}
