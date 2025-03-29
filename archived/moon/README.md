## Moon Module

---

### Environment Variables

In the root directory of your project, three environment variables are required in the `.env` file:

- **MEGA_HOST**: Used for redirecting to public backend routes during the OAuth process.
- **MEGA_INTERNAL_HOST**: Utilized for internal API requests. Given that the application uses SSR and Next.js Route Handlers, this variable allows you to specify a domain name within the container network.

### Environment Handling

- **Development Mode**: Next.js automatically reads the environment variables from the `.env` file.
- **Production Mode**: Environment variables must be passed via Docker commands when deploying the application.

### Development Commands

```bash
pnpm i

pnpm run dev
```
