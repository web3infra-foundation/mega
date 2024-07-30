// 'use client'

export default function LoginLayout({ children }) {
    return (

      <html className="h-full bg-gray-50" lang="en">
        <head>
          <title>Login - Mega</title>
        </head>
        <body className="h-full">
          <div className="login-container">
            {children}
          </div>
        </body>
      </html>
    );
  }
  