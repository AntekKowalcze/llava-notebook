Trzy opcje:

    Zaloguj się do konta online (workspace zespołu)

    Zaloguj się do konta lokalnego (prywatne konto na urządzeniu)

        Nowy użytkownik? Utwórz konto (link na dole)

Ścieżka 1: Logowanie do konta online

    Podaj email + hasło workspace

    Aplikacja pokazuje listę istniejących kont lokalnych na urządzeniu (Jan, Ania, Konto3...)

    Wybierz istniejące konto lokalne ALBO [+ Utwórz nowe konto lokalne] ← link zawsze dostępny

    Synchronizacja danych

    Gotowe - masz dostęp do notatek zespołu + prywatne notatki

Ścieżka 2: Logowanie do konta lokalnego

    Aplikacja pokazuje listę kont lokalnych na urządzeniu (Jan, Ania, Konto3...)

    Wybierz konto ALBO [+ Utwórz nowe konto lokalne] ← link zawsze dostępny na dole listy

    Podaj hasło (jeśli wybrano istniejące) LUB podaj nazwę + hasło (jeśli tworzysz nowe)

    Zalogowano - pracujesz tylko lokalnie (offline)

Ścieżka 3: Nowy użytkownik

Krok 1: Tworzenie konta lokalnego

    Podaj nazwę użytkownika

    Podaj hasło (i potwierdź)

    Konto lokalne utworzone i automatycznie zalogowane

Krok 2: Pytanie o synchronizację

    "Chcesz włączyć synchronizację z workspace?"

    TAK → przejdź do kroku 3

    NIE → pracujesz tylko lokalnie (można włączyć później w ustawieniach)

Krok 3: Tworzenie konta online (jeśli wybrał TAK)

    Podaj email

    Podaj hasło dla konta online (WAŻNE: osobne hasło, zawsze oddzielne od hasła lokalnego dla bezpieczeństwa)

    Potwierdź hasło

    Wyślij email weryfikacyjny

    Po kliknięciu w link - konto online utworzone i połączone z lokalnym

    Synchronizacja uruchomiona




Jeśli pierwsze uruchomienie wariant: 
Utwórz konto lokalnie -> czy chcesz podłączyć konto online? tak -> rejestracja/logowanie online -> email -> zalogowano 
                                                            nie -> pokaż kody -> zalogowano

Nie pierwsze uruchomienie -> Zaloguj konto lokalne -> masz konto online? tak- logowanie online
                                                                         nie - zalogowano



Jeśli nie pierwsze uruchomienie LOGOWANIE ONLINE, LOGOWANIE OFFLINE, utwórz konto ONLINE



REJESTRACJA OFFLINE 
NAZWA
HASŁO
HASŁO POWTÓRZONE 
-> POKAŻ KODY DO ODZYSKANIA HASŁA


REJESTRACJA OFFLNIE
REJESTRACJA OFFLINE +
EMAIL
HASŁO
POWTÓRZONE HASŁO (konto offline coś ala grupowe)
EMAIL WERYFIKACYJNY 
KLIK -> KONTO UTWORZONE I POŁĄCZONE Z LOKALNYM + URUCHAMIAMY SYNC



LOGOWANIE OFFLINE:
nazwa uzytkownika 
hasło


LOGOWANIE ONLINE:
email
hasło do konta online



// Implement local auth system (SQLite + Argon2).

// Add “link to online account” functionality that only syncs metadata at first (email + username).

// Once that works, move to crypto — encrypt stored passkeys.

// Only then integrate Mongo auth logic.

// Pierwsze uruchomienie
// – Poproś o utworzenie konta lokalnego (nazwa + hasło).
// – Utwórz lokalny profil i zakończ — aplikacja działa offline.

// Opcja połączenia z chmurą (dobrowolna)
// – Po założeniu konta zapytaj: „Chcesz połączyć konto z chmurą?” (tak/nie).
// – Jeśli nie — koniec. Jeśli tak — przejdź do rejestracji/logowania online.

// Przypisanie lokalnego profilu do konta online
// – Po zalogowaniu online pokaż listę lokalnych profili na urządzniu (jeśli są).
// – Użytkownik wybiera który profil połączyć lub tworzy nowy lokalny profil powiązany z tym kontem.

// Tryb działania po zalogowaniu
// – Jeśli użytkownik pracuje offline → zmiany są lokalne, synchronizacja wstrzymana.
// – Jeśli pracuje online → zmiany trafiają do kolejki synchronizacji i są wysyłane do chmury, gdy jest połączenie.

// Brak połączenia sieciowego (tryb awaryjny)
// – Informuj użytkownika, że działa offline i że synchronizacja zostanie wykonana przy ponownym połączeniu.
// – Gromadź zmiany w kolejce do wysłania.

// Zmiana hasła / niespójność haseł
// – Zmiana hasła lokalnego nie zmienia hasła online i odwrotnie.
// – Jeśli chce spójność, zaoferuj funkcję „zaktualizuj hasło online z lokalnego” (wyraźna zgoda użytkownika).

// Rozwiązywanie konfliktów danych
// – Na początek stosuj prostą regułę: ostatnia zmiana wygrywa.
// – Daj użytkownikowi możliwość przywrócenia starej wersji (historia) w razie potrzeby.

// Widoczność urządzeń i zarządzanie
// – W online panelu pokaż listę urządzeń / lokalnych profili powiązanych z kontem.
// – Pozwól usuwać urządzenia (odłączenie) i wymuszać ponowne powiązanie.

// Odłączenie konta online
// – Użytkownik może odłączyć konto online od lokalnego profilu — wtedy lokalne dane pozostają, synchronizacja przestaje działać.

// Bezpieczeństwo i prywatność (komunikaty UX)
// – Zawsze informuj, że notatki lokalne pozostają prywatne.
// – Przy funkcji „AI / zewnętrzne API” pros użytkownika o zgodę przed wysłaniem treści.
// – Daj opcję exportu klucza/backup przed operacjami ryzykownymi.

// Przywracanie i migracja urządzeń
// – Przy dodawaniu nowego urządzenia daj możliwość „przywróć z chmury” (ściągnij notatki i powiąż profil).
// – W dokumentacji opisz prostą procedurę odzyskiwania dostępu (np. mail z kodem).

// Monitorowanie i komunikaty
// – Pokazuj status synchronizacji, listę oczekujących operacji i ostatnią udaną synchronizację.
// – Informuj o błędach w sposób konkretny i możliwy do działania (np. „Brak miejsca na koncie S3 — przerwij sync”).


