namespace test {
    func increment(mut:ref to_increment) {
        to_increment = to_increment + 1
    }

    func immut_increment(immut to_increment) {
        ret to_increment + 1
    }

    func hello_world(immut print) {
        std::println(print)
    }
}

namespace outer {
    namespace inner {
        func hello() {
            ret "nested namespace"
        }
    }
}