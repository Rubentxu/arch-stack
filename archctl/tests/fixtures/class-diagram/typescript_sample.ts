// Sample TypeScript module for class-diagram fixture.

interface Named {
    name: string;
}

class Animal implements Named {
    name: string;
    constructor(name: string) {
        this.name = name;
    }
}

class Pet extends Animal {
    owner: string;
}

interface Serializable {
    serialize(): string;
}

class Config implements Serializable {
    data: Record<string, unknown>;
    constructor() {
        this.data = {};
    }
    serialize(): string {
        return JSON.stringify(this.data);
    }
}
