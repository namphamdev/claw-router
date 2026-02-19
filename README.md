# Claw9Router

A hybrid AI router combining features of [ClawRouter](https://github.com/BlockRunAI/ClawRouter) and [9router](https://github.com/decolua/9router).

## Features

*   **Smart Routing**: Routes requests based on tiers (Subscription, Cheap, Free, PayPerRequest) and cost/latency.
*   **Provider Agnostic**: Supports OpenAI, Anthropic, Google, DeepSeek, XAI, and custom providers.
*   **Cost Tracking**: Tracks saved costs and request stats.
*   **Dashboard**: React-based UI to manage providers and view stats.
*   **Local Processing**: Routing logic runs locally for privacy and speed.

## Project Structure

*   `backend/`: Rust Axum server handling API requests and routing logic.
*   `frontend/`: React Vite application for dashboard and settings.

## Getting Started

### Backend

1.  Navigate to `backend/`:
    ```bash
    cd backend
    ```
2.  Run the server:
    ```bash
    cargo run
    ```
    The server will start on `http://127.0.0.1:3000`.

### Frontend

1.  Navigate to `frontend/`:
    ```bash
    cd frontend
    ```
2.  Install dependencies:
    ```bash
    npm install
    ```
3.  Start the development server:
    ```bash
    npm run dev
    ```
    The UI will be available at `http://localhost:5173`.

## Configuration

Configuration is stored in `backend/config.json` (created on first run). You can also manage it via the Frontend Settings page.
