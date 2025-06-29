export const pickWithReflect = <T extends Object, K extends keyof T>(obj: T, keys: K[]): Pick<T, K> => {
  const result = {} as Pick<T, K>

  keys.forEach((i) => {
    result[i] = Reflect.get(obj, i)
  })
  return result
}
