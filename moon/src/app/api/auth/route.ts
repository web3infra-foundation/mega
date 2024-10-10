import { isLoginIn } from '@/app/lib/dal';

export async function GET(request: Request, { params }: { params: { id: string } }) {

  const login = isLoginIn();

  if (!login) {
    return new Response(null, { status: 401 })
  }
  return new Response(null, { status: 200 })
};