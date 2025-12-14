/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        note: {
          graphite: '#0F0F10', // Tło
          garnet: '#9F1239',   // Czerwony akcent
          paprika: '#F97316',  // Pomarańczowy akcent
          glow: '#FACC15',     // Żółty akcent
          pumice: '#E7E5E4',   // Szary 
          ivory: '#FFFBEB',    // Jasny kremowy idealny dla tekstu
        },
      },
      boxShadow: {
        'ember': '0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 0 15px 3px rgba(234, 88, 12, 0.4), 0 0 40px 5px rgba(127, 29, 29, 0.5)',
      },
      fontFamily: {
        sans: ['Outfit', 'sans-serif'],
      },
    },
  },
  plugins: [],
};
