import { isLoginIn } from '@/app/lib/dal';

export async function GET(request: Request) {

  const login = await isLoginIn();
  if (!login) {
    return new Response(null, { status: 401 })
  }
  return new Response(null, { status: 200 })
};