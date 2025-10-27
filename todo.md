1. zrobić funkcje z połączeniem i z retry bo jest podobna moze w parametrze ilość łączeń
2. Dodać id do struct note, aby ułatwić wyszukiwanie, dodawanie na podstawie ostatnia dodana +1 
3. dodać pozostałe CRUD, wyświetlenie, update i delete
4. zrobić menu wyboru.
5. próba łączenia co 30 sekund (zapytać o częstotliwość) jesli nie połączone to save_locally a jeśli połączy insert wszystko z save locally
5. zrobić ak aby tworzyło jednego klienta mongo ?co w wypadku gdy sie nie połaczy?
6. dodanie errors.rs z typami błędów
7. przygotować Note z save_locally do zapisu do pliku,