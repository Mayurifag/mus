import structlog
from starlette.middleware.base import BaseHTTPMiddleware, RequestResponseEndpoint
from starlette.requests import Request
from starlette.responses import PlainTextResponse, RedirectResponse, Response

from mus.config import get_secret_auth_route

logger = structlog.get_logger()

COOKIE_NAME = "mus_session"
COOKIE_MAX_AGE = 315360000  # 10 years in seconds


class AuthMiddleware(BaseHTTPMiddleware):
    def __init__(self, app, **kwargs) -> None:
        super().__init__(app)
        self.secret_route = get_secret_auth_route()
        self.cookie_secret = self.secret_route  # They are the same value

    async def dispatch(
        self, request: Request, call_next: RequestResponseEndpoint
    ) -> Response:
        if not self.secret_route:
            return await call_next(request)

        if request.url.path == self.secret_route:
            response = RedirectResponse("/", status_code=303)
            response.set_cookie(
                COOKIE_NAME,
                self.secret_route,
                max_age=COOKIE_MAX_AGE,
                httponly=True,
                samesite="lax",
                secure=request.url.is_secure,
            )
            return response

        session_cookie = request.cookies.get(COOKIE_NAME)
        if session_cookie != self.secret_route:  # Compare with secret_route directly
            return PlainTextResponse("Forbidden", status_code=403)

        return await call_next(request)
