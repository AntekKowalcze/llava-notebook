
---

# 🧱 Architektura SmartNote

## 🎯 Założenia projektu

* Offline-first notatnik z możliwością synchronizacji przez MongoDB i S3.
* Zapis lokalny w plikach Markdown + metadane w SQLite.
* Pełne szyfrowanie end-to-end (E2E) danych użytkownika.
* Każdy użytkownik posiada osobny profil i lokalną bazę danych.
* Aplikacja desktopowa oparta o **Tauri (Rust + Vue/Svelte)**.

---

## ⚙️ Tech stack

* **Frontend:** Vue + Tauri UI
* **Backend:** Rust (tokio, serde, rusqlite, mongodb, aws-sdk-s3)
* **Lokalna baza:** SQLite (z feature `bundled`)
* **Chmurowa baza:** MongoDB Atlas
* **Przechowywanie plików AWS S3 (zaszyfrowane)
* **Szyfrowanie:** ChaCha20-Poly1305 + Argon2id (E2E)
* **Parser Markdown:** pulldown_cmark / comrak

---

## 🧩 Struktura projektu (moduły Rust)

```
src/
 ├── main.rs            # Punkt wejścia, konfiguracja Tauri
 ├── commands.rs        # #[tauri::command] API dla frontendu
 ├── models/            # Definicje struktur (Note, User, Attachment)
 ├── services/
 │    ├── storage.rs    # CRUD lokalny (Markdown + SQLite)
 │    ├── sync.rs       # Synchronizacja z MongoDB + S3
 │    ├── crypto.rs     # Szyfrowanie / deszyfrowanie danych
 │    ├── auth.rs       # Logowanie i konta użytkowników
 │    ├── attachment.rs # Zarządzanie plikami i ich stanem synchronizacji
 │    ├── cleaner.rs    # Sprzątanie osieroconych plików (S3/local)
 │    └── logger.rs     # Logowanie operacji i błędów
 ├── config.rs          # Ustawienia i ścieżki aplikacji
 └── utils.rs           # Pomocnicze funkcje
```

---

## 🗂️ Struktura danych lokalnych

```
~/.smartnote/
  users/
    <user_email_hash>/
      db.sqlite              # Metadane i historia
      notes/<uuid>.md        # Treść notatek (Markdown)
      assets/<uuid>/...      # Obrazy i załączniki
      keys/master.key        # Zaszyfrowany klucz główny
      config.json
      logs/app.log
```

---

## 📄 Model danych


### ☁️ MongoDB (chmurowa baza) 

W pliku data models
## 🔐 Bezpieczeństwo

* Szyfrowanie end-to-end (E2E): notatki i pliki szyfrowane lokalnie.
* Klucz główny tworzony z hasła użytkownika (Argon2id).
* Przechowywanie zaszyfrowanego master keya lokalnie (`keys/master.key`).
* S3 i MongoDB widzą wyłącznie zaszyfrowane dane.
* Utrata klucza = brak możliwości odszyfrowania (wymagany backup).

---

## 🔄 Synchronizacja

* Tryb **offline-first** — wszystko działa lokalnie, nawet bez internetu.
* **SyncManager** co 30 sekund lub po `Ctrl+S`:

  1. Wysyła lokalne zmiany do Mongo/S3 (upsert per `local_id`).
  2. Pobiera zdalne zmiany (`updated_at` > `last_sync`).
  3. Rozwiązuje konflikty (last-writer-wins + snapshot historii).
  4. Emituje eventy `sync:started`, `sync:progress`, `sync:completed`.
* Kolejka `sync_ops` w SQLite przechowuje wszystkie oczekujące zmiany.

---

## 🧹 Zarządzanie załącznikami

* Każdy załącznik ma `checksum_encrypted` i `sync_state`.
* Upload do S3 odbywa się po szyfrowaniu (streaming).
* Deterministyczne klucze S3 (`user_id/local_id/attachment_id`).
* `AttachmentCleaner` usuwa osierocone pliki lokalne i w S3.

---

## 🧠 Mechanizmy dodatkowe

* **History snapshots:** każda edycja notatki zapisuje wersję lokalnie (`history/<note_id>/v{n}.md`).
* **Logger:** loguje operacje CRUD, sync, crypto, błędy.
* **Tauri Event System:** komunikacja Rust ↔ frontend (`sync:started`, `note:created`, `error:network`).
* **Offline mode:** użytkownik może wyłączyć synchronizację ręcznie.

---

## 🧰 Technologie wspomagające

* `rusqlite` – lokalna baza danych (z feature `bundled`)
* `mongodb` – klient MongoDB
* `aws-sdk-s3` – upload i download zaszyfrowanych plików
* `tokio` – asynchroniczność
* `uuid`, `chrono`, `serde`, `argon2`, `chacha20poly1305`

---

## 🚀 Plan rozwoju (iteracyjny)

1. Lokalny CRUD (Markdown + SQLite)
2. GUI (Tauri + Vue/Svelte)
3. Podstawowy sync notatek z MongoDB
4. Lokalny AttachmentManager + upload do S3
5. E2E szyfrowanie notatek i załączników
6. Logger + historia wersji + konflikt manager
7. Integracja eventów Tauri i powiadomień w UI

---

## 📊 Kluczowe zasady projektowe

* Każdy komponent ma **jedną odpowiedzialność** (SRP).
* Dane **zawsze najpierw lokalnie**, później sync.
* Synchronizacja **idempotentna** (bez duplikatów).
* Brak serwera pośredniego — klient sam szyfruje i wysyła dane.
* Kod i logika przygotowane pod **multi-device sync**.

        ┌────────────────────────────┐
        │        🖥️  Frontend        │
        │  Vue / Svelte (Tauri UI)   │
        │                            │
        │  • Edycja Markdown         │
        │  • Podgląd live preview    │
        │  • Lista notatek / tagi    │
        │  • Powiadomienia i eventy  │
        └──────────────┬─────────────┘
                       │
                 Tauri Commands + Event System
                       │
                       ▼
┌────────────────────────────────────────────────────┐
│              ⚙️  Rust Backend (Tauri)              │
│----------------------------------------------------│
│  AppState (shared state)                           │ 
│  ├── StorageService    CRUD na plikach i SQLite    │ 
│  ├── SyncManager       Synchronizacja z Mongo/S3   │   
│  ├── CryptoService     Szyfrowanie / deszyfrowanie │ 
│  ├── AttachmentManager Zarządzanie załącznikami    │ 
│  ├── AuthService       Logowanie / klucz master    │   
│  ├── Cleaner           Usuwanie orphaned plików    │
│  └── Logger            Audyt i diagnostyka         │  
└──────────────────────┬─────────────────────────────┘
                       │
                       ▼
┌───────────────────────────────────────────┐
│       💾  Lokalny Storage (Offline)       │
│-------------------------------------------│
│~/.smartnote/users/<user>/                 │                        
│ ├── notes/*.md  Markdownowe notatki       │                     
│ ├── assets/*    Obrazy i załączniki       │
│ ├── db.sqlite   Metadane i historia       │
│ ├── keys/master.key zyfrowany klucz główny│
| └── logs/app.log   Dziennik operacji      │
│ |__ tmp/        Pliki w trakcie zapisu    │
│ |__ delete_tmp/ notatki po soft delete    │
│                                           │
│  → Zapis offline-first                    │
│  → Szyfrowanie E2E                        │
└───────────────────┬───────────────────────┘
                    |
                    │
                   Sync (co 30s / Ctrl + S )
                    │
                    ▼
┌───────────────────────────────────────────┐
│             ☁️  Cloud Backend             │
│-------------------------------------------│
│ MongoDB Atlas -> Metadane notatek (JSON)  │                        
│ S3 Storage -> Zaszyfrowane pliki i obrazy │                     
│                                           │
│          🔒 Wszystko zaszyfrowan          │
│           → ChaCha20-Poly1305             │
│           → Klucz z Argon2id              │
│                                           │
│      🧹 Cleaner dba o spójność            │
│         (usuwa orphaned pliki)            │
└───────────────────┬───────────────────────┘
                    │
                    ▼
  ┌──────────────────────────────────┐
  │ 🔄  Synchronizacja dwukierunkowa |
  │----------------------------------|
  │  1. Zmiany lokalne → Mongo/S3    │  
  │  2. Zmiany zdalne → lokalny cache│  
  │  3. Konflikty → last-writer-wins │  
  │  4. Historia i snapshoty         │  
  │  5. Eventy do UI (progress, err) │  
  └──────────────────────────────────┘


Summary made by local AI hosted on own server with privacy politics, and more ai features


6. zrobić connection reuse przez connection struct jakoś i implementacje + active user żeby nie deserializować
8. zaimplementować checksum_ecrypted w bazie danych
10. dodać logger
11. Hardcoded Magic Values (MAINTAINABILITY) (popros perplexity o wypisanie z kodu)
12. limit prób w hasłach
13. testy integracyjne
 myśle że 1-2 punkty dziennie średnio to dobry wynik, jedne są krótsze, drugie dużo dłuższe