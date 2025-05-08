# Mus Next Generation

## Introduction

*To be completed: Brief overview of the Mus Next Generation project.*

## Features

- Modern desktop and web music server
- Universal app with Tauri 2.0
- Vite/React/TypeScript frontend
- FastAPI/SQLModel backend
- Secure authentication for web access
- Dockerized development and production

## Tech Stack

- **Frontend:** Vite, React, TypeScript, Zustand, Tailwind CSS, Shadcn/UI, TanStack Query, date-fns
- **Backend:** FastAPI, SQLModel, Uvicorn, python-jose, Pytest, HTTPX
- **Desktop:** Tauri 2.0
- **Containerization:** Docker, Docker Compose

## Getting Started

### Setup

1. **Backend**

   Copy the sample environment variables:

   ```bash
   cd backend
   cp .env.example .env
   ```

   Edit the `.env` file and set the following variables:

   ```
   # Authentication Settings
   AUTH_SECRET_KEY=your-super-secure-jwt-signing-key-change-in-production
   LOGIN_SECRET=your-login-secret-change-in-production

   # Database
   DATABASE_URL=sqlite:///./mus.db
   ```

   Install dependencies:

   ```bash
   cd backend
   uv venv
   source .venv/bin/activate
   uv pip install -e .
   ```

2. **Frontend** (to be completed)

### Running

1. **Backend**

   ```bash
   cd backend
   source .venv/bin/activate
   uvicorn src.mus.main:app --reload
   ```

   The backend will be available at http://localhost:8000

   You can log in via the web interface by visiting:
   http://localhost:8000/api/v1/auth/login-via-secret/your-login-secret-change-in-production

2. **Frontend** (to be completed)

## Project Structure

```
backend/    # FastAPI backend (Hexagonal Architecture)
frontend/   # Vite/React/TS frontend (SPA, Tauri shell)
docker/     # Dockerfiles, Compose, makefiles
```

## Usage

*To be completed: Usage instructions and example workflows.*
