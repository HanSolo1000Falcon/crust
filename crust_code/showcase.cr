get ./component.cr

entry {
    std::println("== crust showcase ==")

    # literals, arithmetic, mixed numeric operations, and comparisons
    immut integer = 7
    immut decimal = 2.5
    immut sum = integer + decimal
    immut power = 2 ^ 3.0
    immut comparison = sum > 9
    immut absolute = std::abs(-3)
    immut smaller = std::min(4, 9)
    immut larger = std::max(4, 9)
    std::println("numbers: {integer}, {decimal}, {sum}, {power}, {comparison}, abs={absolute}, min={smaller}, max={larger}")

    # booleans, blocks, and if/else
    if comparison && true {
        std::println("branch: true")
    } else {
        std::println("branch: false")
    }

    {
        immut scoped = "inside a block"
        std::println("scope: {scoped}")
    }

    # while, continue, break, and mutation of an outer variable
    mut counter = 0
    while counter < 5 {
        counter = counter + 1
        if counter == 2 {
            continue
        }
        if counter == 4 {
            break
        }
        std::println("loop counter: {counter}")
    }
    std::println("loop finished at: {counter}")

    # namespace functions, ordinary functions, and reference parameters
    mut referenced = 10
    test::increment(referenced)
    immut incremented = test::immut_increment(referenced)
    std::println("reference: {referenced}, returned: {incremented}")
    test::hello_world("namespace call")
    immut nested_message = outer::inner::hello()
    std::println("nested: {nested_message}")

    # objects, constructors, private fields, and private methods
    immut computed = addition::construct(1, 2):>compute()
    std::println("object result: {computed}")

    immut record = record::construct("crust"):>read()
    std::println("immutable field: {record}")

    # arrays: construct, len, at, push, and rm
    mut values = array::construct("a", "b")
    values:>push("c")
    values:>set(0, "updated")
    immut before_remove = values:>len()
    immut first = values:>at(0)
    immut removed = values:>rm(1)
    immut after_remove = values:>len()
    std::println("array: first={first}, removed={removed}, lengths={before_remove}/{after_remove}")

    # strings and character arrays
    immut text = "Crust"
    immut text_len = text:>len()
    immut first_character = text:>at(0)
    immut characters = text:>as_char_array()
    immut character_count = characters:>len()
    std::println("string: {text}, first={first_character}, length={text_len}, chars={characters}, char_count={character_count}")

    # environment arguments and a numeric standard-library call
    immut arguments = env::args()
    immut has_arguments = env::has_args()
    immut random_value = random(0, 10)
    std::println("args: {arguments}, has_args={has_arguments}, random: {random_value}")
}

func random(min, max) {
    ret std::rand(min, max)
}

object addition {
    mut:private a
    mut:private b

    constructor(immut a, immut b) {
        this:>a = a
        this:>b = b
    }

    func compute() {
        ret this:>compute_private()
    }

    func:private compute_private() {
        ret this:>a + this:>b
    }
}

object record {
    :private value

    constructor(immut value) {
        this:>value = value
    }

    func read() {
        ret this:>value
    }
}