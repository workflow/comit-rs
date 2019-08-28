export default class Unsupported extends Error {
    constructor(what: string) {
        super(`${what} is not supported by this test suite`);
    }
}
