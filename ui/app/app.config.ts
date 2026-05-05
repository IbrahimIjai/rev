export default defineAppConfig({
  ui: {
    colors: {
      primary: 'orange',
      secondary: 'purple',
      success: 'green',
      info: 'blue',
      warning: 'amber',
      error: 'red',
      neutral: 'slate'
    },
    button: {
      slots: {
        base: 'font-semibold rounded-xl'
      }
    },
    card: {
      slots: {
        root: 'rounded-2xl shadow-sm'
      }
    },
    input: {
      slots: {
        root: 'rounded-xl'
      }
    }
  }
})
