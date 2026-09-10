namespace env {
    func has_args() {
        ret env::args():>len() > 0
    }
}
