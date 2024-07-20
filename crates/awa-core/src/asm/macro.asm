#fn repeat(code, count) => {
    count <= 0 ? 0`0 : {
        code @
        repeat(code, count - 1)
    }
}
#fn awascii(string, size) => {
    size <= 0 ? 0`0 : {
        chr = char(string)

        asm { blo {chr} } @
        awascii(string >> 8, size - 1)
    }
}

#ruledef macro {
    ; ( a:count -- )
    pop {count} => {
        assert(count > 0)
        repeat(asm { pop }, count)
    }
    ; ( a -- a:count a )
    dpl {count} => {
        assert(count > 0)
        repeat(asm { dpl }, count)
    }
    ; ( a:count -- [a...; count] )
    mrg {count} => {
        assert(count >= 2)
        repeat(asm { mrg }, count - 1)
    }
    ; ( a:count b:distance -- b a )
    sbm {distance: u5}, {count} => {
        assert(distance > 0)
        total = distance + count - 1
        assert(total > 0)
        repeat(asm { sbm {total} }, count)
    }
    ; ( a:count b:? -- b a )
    sbm {distance: u5}, {count} => {
        assert(distance == 0)
        repeat(asm { sbm 0 }, count)
    }
    ; ( -- [string...; size] )
    blo {string}, {size} => {
        assert(size >= 1)
        assert(size < 32)

        awascii(string, size) @
        asm { srn {size} }
    }
    ; ( count -- i count )
    loop {label: u5} => asm {
        blo 0
        lbl {label}
        blo 1
        4dd
    }
    ; ( i count -- )
    end loop {label: u5} => asm {
        lss
        jmp {label}
        pop 2
    }
}