1. Przy dodaniu logownia, sprawdzać czy użytkowik loguje się pierwszy raz (albo sie nie loguje i jest jakis local user) i wtedy raz próbować tworzyć ścieżki i jeśli sie nie uda po fallbacku próbować robić to ponownie, fallback można czytać z config.json, a jeśli sie uda to zapisywać ścieżki do pliku path.jsoni tylko odczytywac je przy starie programu a nie próbować tworzyć od nowa i pobierać znowu, tylko wtedy jest problem ze zmianą ścieżki dla użytkownika, ale wtedy trzeba po logowaniu dawać ścieżke i fallback oraz path.json tam, musze do tego dodać sprawdzanie dostępu i jeśli nie ma to robić fallback


2. dawać komunikat dla usera w taurii przy każdej rzeczy która powoduje exit z programu

