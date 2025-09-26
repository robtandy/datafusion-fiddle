import React, { useState } from 'react';

export interface SqlRequest {
  stmts: string[]
  distributed: boolean
}

export interface SqlResponse {
  columns: Array<[string, string]>,
  rows: Array<Array<string>>,
  logical_plan: string
  physical_plan: string
}

export async function executeStatements (stmts: string[], distributed: boolean): Promise<SqlResponse> {
  const req: SqlRequest = {
    stmts,
    distributed
  }
  // Support configurable API URL for Lambda deployment
  const apiUrl = import.meta.env.VITE_API_URL || '';
  const endpoint = apiUrl ? `${apiUrl}/api/sql` : '/api/main';

  const res = await fetch(
    endpoint,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    }
  )
  if (res.status === 200) {
    return await res.json()
  } else if (res.status === 400) {
    const { message } = await res.json()
    throw new Error(message)
  } else {
    const msg = await res.text()
    throw new Error(`unexpected status ${res.status}: ${msg}`)
  }
}

export type ApiState =
  { type: 'nothing' } |
  { type: 'loading' } |
  { type: 'error', message: string } |
  { type: 'result', result: SqlResponse }

export interface ApiRequest {
  statement: string
  distributed: boolean
}

export function useApi () {
  const [state, setState] = useState<ApiState>({ type: 'nothing' });

  const execute = React.useCallback(async (req: ApiRequest) => {
    setState({ type: 'loading' });
    const result = await executeStatements(
      req.statement.split(';').map(_ => _.trim()).filter(_ => _.length > 0),
      req.distributed
    )
      .then((result) => ({ type: 'result' as const, result }))
      .catch((err) => ({ type: 'error' as const, message: err.toString() }));
    setState(result)
    return result
  }, []);

  return { state, execute };
}