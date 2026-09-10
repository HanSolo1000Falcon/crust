namespace std {
    func abs(value) {
        if value < 0 {
            ret -value
        }
        ret value
    }

    func min(left, right) {
        if left < right {
            ret left
        }
        ret right
    }

    func max(left, right) {
        if left > right {
            ret left
        }
        ret right
    }
}
