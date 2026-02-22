// Tailwind CSS configuration with bronze/copper design system
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        bronze: {
          50: '#faf8f5',
          100: '#f4f0e8',
          200: '#e7ddd0',
          300: '#d5c4ad',
          400: '#c0a586',
          500: '#b08968',
          600: '#a3765c',
          700: '#88604d',
          800: '#6f4e42',
          900: '#5b4137',
          950: '#32211c',
        },
        copper: {
          50: '#fdf8f6',
          100: '#f2e8e5',
          200: '#eaddd7',
          300: '#e0cec7',
          400: '#d2bab0',
          500: '#bfa094',
          600: '#a18072',
          700: '#977669',
          800: '#846358',
          900: '#43302b',
        },
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace'],
      },
      keyframes: {
        fadeIn: {
          '0%': { opacity: '0', transform: 'translateY(-4px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
      },
      animation: {
        'fade-in': 'fadeIn 150ms ease-out',
      },
    },
  },
  plugins: [
    require('@tailwindcss/forms'),
  ],
}
