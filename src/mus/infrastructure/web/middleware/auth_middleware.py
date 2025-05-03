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
        self.cookie_secret = self.secret_route
        logger.info(
            "AuthMiddleware initialized", secret_route_present=self.secret_route
        )

    async def dispatch(
        self, request: Request, call_next: RequestResponseEndpoint
    ) -> Response:
        # Temporary diagnostic logging for proxy headers
        logger.info(
            "Request headers",
            x_forwarded_proto=request.headers.get("x-forwarded-proto"),
            x_forwarded_for=request.headers.get("x-forwarded-for"),
            x_forwarded_host=request.headers.get("x-forwarded-host"),
            scheme=request.scope.get("scheme"),
        )

        request_path = request.url.path.strip("/")
        logger.info(
            "AuthMiddleware received request",
            path=request_path,
            method=request.method,
            client=request.client,
            scheme=request.scope.get("scheme"),
        )

        if not self.secret_route:
            return await call_next(request)

        if request_path == self.secret_route:
            secure_flag = request.scope.get("scheme") == "https"
            logger.info(
                "Setting auth cookie via secret route",
                path=request_path,
                secure_flag=secure_flag,
                client=request.client,
            )
            response = RedirectResponse("/", status_code=303)
            response.set_cookie(
                COOKIE_NAME,
                self.secret_route,
                max_age=COOKIE_MAX_AGE,
                httponly=True,
                samesite="lax",
                secure=secure_flag,
                path="/",
            )
            return response

        session_cookie = request.cookies.get(COOKIE_NAME)
        logger.info(
            "Checking auth cookie",
            path=request_path,
            cookie_value=session_cookie,
            expected_value=self.secret_route,
            client=request.client,
        )

        if session_cookie != self.secret_route:
            logger.warning(
                "Auth failed: cookie mismatch or missing",
                path=request_path,
                cookie_value=session_cookie,
                client=request.client,
                wanted=self.secret_route,
                scheme=request.scope.get("scheme"),
            )
            return PlainTextResponse("Forbidden", status_code=403)

        logger.info(
            "Auth succeeded: cookie matched", path=request_path, client=request.client
        )
        return await call_next(request)
