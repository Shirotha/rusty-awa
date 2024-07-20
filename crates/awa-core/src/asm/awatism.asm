#ruledef awatism {
    nop => 0x00`5
    prn => 0x01`5
    pr1 => 0x02`5
    red => 0x03`5
    r3d => 0x04`5
    trm => 0x1F`5
    blo {value: s8} => 0x05`5 @ value
    sbm {distance: u5} => 0x06`5 @ distance
    pop => 0x07`5
    dpl => 0x08`5
    srn {count: u5} => 0x09`5 @ count
    mrg => 0x0A`5
    4dd => 0x0B`5
    sub => 0x0C`5
    mul => 0x0D`5
    div => 0x0E`5
    cnt => 0x0F`5
    lbl {label: u5} => 0x10`5 @ label
    jmp {label: u5} => 0x11`5 @ label
    eql => 0x12`5
    lss => 0x13`5
    gr8 => 0x14`5
    p0p => 0x16`5
}