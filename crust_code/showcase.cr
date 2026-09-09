get ./component.cr

entry {
    mut incrementable = 2
    test::increment(incrementable)
    std::println("current: {incrementable}")
    incrementable = test::immut_increment(incrementable)
    std::println("current: {incrementable}")

    mut c = 'c'

    if std::rand(0, 1) > 0.5 {
        std::println("it was more than 0.5")
    } else {
        std::println("it wasn't more than 0.5")
    }

    immut addition_object = addition::construct(1, 2)
    std::println("result: {addition_object:>compute()}")

    while incrementable < 100 {
        incrementable = incrementable + std::rand(0, 1)
        std::println("{incrementable}")
    }

    mut arr = array::construct("a", "b")
    {
        mut i = 0
        while i < arr:>len() { # no for loops because im lazy and everything can be done with while loops
            std::println("{i}: {arr:>at(i)}")
            i = i + 1
        }
    }
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