export interface RuntimeInstallError {
  message: string;
  details: string;
}

export function createRuntimeInstallError(error: unknown, runtimeName: string): RuntimeInstallError {
  const details = error instanceof Error ? error.message : String(error);
  const lowerDetails = details.toLowerCase();

  let reason = 'The downloaded runtime appears to be corrupted or incomplete.';

  if (lowerDetails.includes('checksum')) {
    reason = 'The downloaded runtime failed verification and was not installed.';
  } else if (lowerDetails.includes('html') || lowerDetails.includes('text/') || lowerDetails.includes('instead of a runtime archive')) {
    reason = 'The download server returned a page instead of the runtime archive.';
  } else if (lowerDetails.includes('http 404') || lowerDetails.includes('not found')) {
    reason = 'The runtime archive could not be found on the download server.';
  } else if (lowerDetails.includes('incomplete') || lowerDetails.includes('unexpectedly small')) {
    reason = 'The downloaded runtime appears to be incomplete.';
  } else if (lowerDetails.includes('network') || lowerDetails.includes('connection') || lowerDetails.includes('timeout')) {
    reason = 'The runtime download could not complete because of a network problem.';
  }

  return {
    message: `Couldn't install ${runtimeName}.\n\n${reason}`,
    details,
  };
}
