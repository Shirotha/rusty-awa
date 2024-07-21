# maps list of values to list of all combinations (ignoring order, including empty)
def combinations [
]: list -> list<list> {
    let values = $in
    mut result = [[]]
    for value in $values {
        let with_value = $result | each { append $value }
        $result = ($result | append $with_value)
    }
    $result
}
# join two sets of values into a list of all possible pairs
def join [
    rhs: list
]: list -> list<list> {
    each {|left| $rhs | each {|right| [$left $right] | flatten} } | flatten
}
# build arguments for a single benchmark
def fragment [
    command: string
    --name (-n): string
    --features (-f): list<string>
]: any -> list {
    let stdin = $in
    let result = if ($name | is-not-empty) {
        [-n $name]
    } else { [] }
    $result | append [
        -p
        $"cargo build --profile release --features ($features | str join ',')"
        $"'($stdin)' | ($command)"
    ]
}
# build all arguments
def build-arguments [
    command: string
    features: record
    --options: record
]: any -> list {
    let stdin = $in | default [] | str join "\n" | $in + "\n"
    [
        -S nu
    ] | append ($options
        | default {}
        | items {|name, value| [$"--($name)" $value] }
        | flatten
    ) | append ($features
        | items {|name, features| $stdin | fragment $command -n $name -f $features}
        | flatten
    )
}
# get all combinations of possible features
def all-features [
]: nothing -> record {
    let abyss = [
        ""
        buffered-
    ] | join [
        linked
    ] | each { str join | "awa-abyss/default_" + $in }
    let features = [
        awa-abyss/cache_count
    ] | combinations
    $abyss
        | join $features
        | each {|list|
            let str = $list | str join ','
            $str | wrap $str
        }
        | reduce {|a, b| $a | merge $b }
}
# perform all benchmarks
def main [
] {
    let commands = [
        {
            arguments: [999999]
            target: "prime.awa"
            options: {
                warmup: 3
            }
        }
        {
            arguments: [50]
            target: "fibs.tism"
            options: {
                warmup: 3
                min-runs: 20
            }
        }
    ]
    let features = all-features
    for cmd in $commands {
        print $"(ansi white_bold)Example:(ansi reset) ($cmd.target)"
        let command = $"target/release/awa run examples/($cmd.target)"
        let options = $cmd.options? | default {}
        let args = $cmd.arguments | build-arguments $command $features --options $options
        ^hyperfine ...$args
    }
}