#!/bin/bash

# Start both backend and frontend for claw-router

cleanup() {
  echo "Shutting down..."
  kill $BACKEND_PID $FRONTEND_PID 2>/dev/null
  wait $BACKEND_PID $FRONTEND_PID 2>/dev/null
  exit 0
}

trap cleanup SIGINT SIGTERM

cd "$(dirname "$0")"

# Start backend (Rust Axum server on port 3000)
echo "Starting backend..."
cd backend
cargo run &
BACKEND_PID=$!
cd ..

# Start frontend (Vite dev server on port 5173)
echo "Starting frontend..."
cd frontend
npm run dev &
FRONTEND_PID=$!
cd ..

echo "Backend PID: $BACKEND_PID (http://localhost:3000)"
echo "Frontend PID: $FRONTEND_PID (http://localhost:5173)"
echo "Press Ctrl+C to stop both."

wait
