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

          graphite: '#0F0F10', // Background
          garnet: '#9F1239',   //Red accent
          paprika: '#F97316',  // Orange accent
          glow: '#FACC15',     // Yellow accent
          pumice: '#E7E5E4',   // Grey 
          ivory: '#FFFBEB',    //  Light ideal for text
        },
      },
      boxShadow: {
        'ember': '0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 0 15px 3px rgba(234, 88, 12, 0.4), 0 0 40px 5px rgba(127, 29, 29, 0.5)',
      },
      dropShadow: {
        'ember': [
          '0 0 6px rgba(234, 88, 12, 0.7)',
          '0 0 20px rgba(127, 29, 29, 0.6)',
          '0 0 40px rgba(250, 204, 21, 0.25)',
        ]
      },

      fontFamily: {
        sans: ['Outfit', 'sans-serif'],
      },
    },
  },
  plugins: [

  ],
};
