package e2e_test

import (
	"testing"

	"github.com/stretchr/testify/suite"
)

func TestMarketplaceTestSuite(t *testing.T) {
	suite.Run(t, new(SGTestSuite))
}
