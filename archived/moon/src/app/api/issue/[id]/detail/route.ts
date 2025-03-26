type Params = Promise<{ id: string }>

export async function GET(request: Request, props: { params: Params }) {
  const params = await props.params

  const endpoint = process.env.MEGA_INTERNAL_HOST;
  const res = await fetch(`${endpoint}/api/v1/issue/${params.id}/detail`)
  const data = await res.json()

  return Response.json({ data })
}