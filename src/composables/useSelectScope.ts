import { computed, inject, provide, watch, type InjectionKey, type Ref } from 'vue'

const selectScope: InjectionKey<Readonly<Ref<string>>> = Symbol('select-scope')

/** 页面和设置分区使用 v-show 保留草稿，切换时仍需关闭挂载到 body 的弹层。 */
export function provideSelectScope(value: Readonly<Ref<string>>) {
  const parent = inject(selectScope, undefined)
  provide(
    selectScope,
    computed(() => `${parent?.value ?? ''}/${value.value}`),
  )
}

export function onSelectScopeChange(close: () => void) {
  const scope = inject(selectScope, undefined)
  if (scope) watch(scope, close, { flush: 'sync' })
}
