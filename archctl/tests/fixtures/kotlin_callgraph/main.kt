package smoke

class Server(val name: String) {
    fun getName(): String = name

    fun handle(req: Request) {
        validate(req)
        process(req)
    }

    private fun validate(req: Request) {
        requireNotNull(req)
    }

    private fun process(req: Request) {
        log("processing")
    }

    private fun log(msg: String) {
        println(msg)
    }
}