export default async function rejectAfter<T>(timeout: number): Promise<T> {
    return new Promise((_, reject) => setTimeout(reject, timeout));
}
