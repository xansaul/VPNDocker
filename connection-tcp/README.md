# Connection TCP Project

Este proyecto Rust implementa una conexión TCP simple con una arquitectura Hub-Cliente, desplegable mediante Docker Compose.

## Requisitos Previos

- Docker
- Docker Compose

## Ejecución

### 1. Nodo HUB (Servidor)

Para levantar el nodo HUB en el servidor, ejecuta el siguiente comando en la raíz del proyecto:

```bash
docker-compose -f docker-compose.hub.yml up --build
```

Esto iniciará el servicio del HUB escuchando en el puerto configurado (por defecto 7878).

### 2. Nodo Cliente

Para levantar el cliente, sigue estos pasos:

1.  Crea un archivo `.env` basándote en el ejemplo provided:
    ```bash
    cp .env.example .env
    ```
2.  Edita el archivo `.env` y modifica la variable `SERVER_ADDR` con la dirección IP y puerto del servidor HUB.

    ```bash
    SERVER_ADDR=192.168.1.XX:7878
    ```

3.  Ejecuta el siguiente comando para iniciar el cliente:

```bash
docker-compose -f docker-compose.client.yml up --build
```
