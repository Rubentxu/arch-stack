package smoke;

public class Server {
    private final String name;

    public Server(String name) {
        this.name = name;
    }

    public String getName() {
        return name;
    }

    public void handle(Request req) {
        validate(req);
        process(req);
    }

    private void validate(Request req) {
        Objects.requireNonNull(req);
    }

    private void process(Request req) {
        log("processing");
    }

    private void log(String msg) {
        System.out.println(msg);
    }
}