// Command constlit-oracle writes the TSV fixtures that constlit-macros' tests
// compare against: Kubernetes' own answers for quantities, names and selectors.
//
// Regenerate from this directory with
//
//	go run . quantity > ../src/testdata/k8s_quantity.tsv
//	go run . names > ../src/testdata/k8s_names.tsv
//	go run . selector > ../src/testdata/k8s_selector.tsv
//
// Inputs never contain tabs or newlines, so each line splits on tabs.
package main

import (
	"fmt"
	"math/rand"
	"os"
	"runtime/debug"
	"strings"

	"k8s.io/apimachinery/pkg/api/resource"
	"k8s.io/apimachinery/pkg/labels"
	"k8s.io/apimachinery/pkg/util/validation"
)

func apimachineryVersion() string {
	info, ok := debug.ReadBuildInfo()
	if ok {
		for _, dep := range info.Deps {
			if dep.Path == "k8s.io/apimachinery" {
				return dep.Version
			}
		}
	}
	return "unknown"
}

func main() {
	if len(os.Args) != 2 {
		fmt.Fprintln(os.Stderr, "usage: constlit-oracle quantity|names|selector")
		os.Exit(2)
	}
	fmt.Printf("# k8s.io/apimachinery %s\n", apimachineryVersion())
	switch os.Args[1] {
	case "quantity":
		quantities()
	case "names":
		names()
	case "selector":
		selectors()
	default:
		fmt.Fprintln(os.Stderr, "unknown subcommand", os.Args[1])
		os.Exit(2)
	}
}

// Quantity inputs: the plan's tables plus generated ones.
var fixedQuantities = []string{
	"", " 1Gi", "1 Gi", "Gi", ".", "+", "x", "1K", "1g", "1t", "1p", "1mi", "1KI", "1gi",
	"1KiB", "1MB", "1GiB", "1e", "1E+", "1e-", "1e3.5", "1e3x", "1.2.3", "1µ", "0.1m", "100n",
	"1.0005", "-0.1m", "8Ei", "-8Ei", "1e400", "9223372036854775808", "1e-400",
	"-1", "2Gi", "1Ki", "-1Gi", "500m", "2000m", "1500m", "5u", "5n", "1.5", "0.1Ki",
	"1", "1.5Gi", "128Mi", "1e3", "1E3", "1E+3", "1e-3", "1E", "1Ei", "+1k", "-1.5", ".5", "5.",
	"1000000n", "0000000000000000000000000001", "1.000000000000000000000000000000",
	"1000000000000000000000000n", "9223372036854775807", "0e-400", "7Ei", "-1.5Gi",
	"250m", "512Mi", "1G", "0.5Ki", "0", "2", "1024", "0.5", "1m", "-0", "+0.5m",
	"9223372036854775807000m", "9223372036854775807.001", "-9223372036854775807",
}

func quantities() {
	r := rand.New(rand.NewSource(1))
	suffixes := []string{"", "n", "u", "m", "k", "M", "G", "T", "P", "E", "Ki", "Mi", "Gi", "Ti",
		"Pi", "Ei", "e3", "E3", "e-3", "e+2", "E-12", "e19", "e-30", "K", "mi", "KB", "i", "e", "x"}
	digits := func(n int) string {
		var b strings.Builder
		for i := 0; i < n; i++ {
			b.WriteByte(byte('0' + r.Intn(10)))
		}
		return b.String()
	}
	inputs := append([]string{}, fixedQuantities...)
	for i := 0; i < 3000; i++ {
		s := []string{"", "+", "-"}[r.Intn(3)] + digits(r.Intn(26))
		if r.Intn(2) == 0 {
			s += "." + digits(r.Intn(13))
		}
		s += suffixes[r.Intn(len(suffixes))]
		inputs = append(inputs, s)
	}
	for _, s := range inputs {
		q, err := resource.ParseQuantity(s)
		if err != nil {
			fmt.Printf("%s\terr\n", s)
			continue
		}
		fmt.Printf("%s\tok\t%s\t%d\n", s, q.AsDec().String(), q.Value())
	}
}

// Name inputs: the plan's tables and edge cases plus generated ones.
var fixedNames = []string{
	"", "My-App", "my_app", "my.app", "-app", "app-", "1web", "web-", "app..config", ".app",
	"app.", "app.-x", "app.my_x", "a/b/c", "/name", "example.com/", "Example.com/name", "_app",
	"app@1", "-web", "web_", "web app", "my-app", "a", "0", "web-1", "app.config.v1",
	"app.kubernetes.io/name", "App.Kubernetes", "example.com/Web_App-1.x", "Web", "v1.2_3",
	" app", "app ", "kube-system", "a.b/c", "UPPER", "x.Y/z",
}

func names() {
	r := rand.New(rand.NewSource(2))
	alphabet := "abcxyzABZ019-._/@ "
	valid := "abcxyz019"
	str := func(chars string, n int) string {
		var b strings.Builder
		for i := 0; i < n; i++ {
			b.WriteByte(chars[r.Intn(len(chars))])
		}
		return b.String()
	}
	kinds := []struct {
		name  string
		check func(string) []string
	}{
		{"dns_label", validation.IsDNS1123Label},
		{"dns1035_label", validation.IsDNS1035Label},
		{"dns_subdomain", validation.IsDNS1123Subdomain},
		{"label_key", validation.IsQualifiedName},
		{"label_value", validation.IsValidLabelValue},
	}
	for _, kind := range kinds {
		inputs := append([]string{}, fixedNames...)
		for _, n := range []int{62, 63, 64, 252, 253, 254} {
			inputs = append(inputs, str(valid, n), "x"+str(valid+".", n-2)+"x", "a/"+str(valid, n))
		}
		inputs = append(inputs, strings.Repeat("a", 64), strings.Repeat("a", 254))
		for i := 0; i < 400; i++ {
			inputs = append(inputs, str(alphabet, r.Intn(71)))
		}
		// Mostly-valid short names, so accepted inputs are well covered too.
		for i := 0; i < 600; i++ {
			var b strings.Builder
			for j := 0; j < 1+r.Intn(15); j++ {
				if r.Intn(5) == 0 {
					b.WriteString(string([]rune("-._/A@é")[r.Intn(7)]))
				} else {
					b.WriteByte(valid[r.Intn(len(valid))])
				}
			}
			inputs = append(inputs, b.String())
		}
		for _, s := range inputs {
			verdict := "ok"
			if len(kind.check(s)) > 0 {
				verdict = "err"
			}
			fmt.Printf("%s\t%s\t%s\n", kind.name, s, verdict)
		}
	}
}

// Selector inputs: the plan's table and examples plus mutated valid selectors.
var fixedSelectors = []string{
	"", " app=web", "app=web,", "app=web,,tier=db", ",app=web", "App@=web", "app=web app",
	"app=-web", "app in (a,)", "app in (,a)", "app in (a,,b)", "app in ()", "app notin ()",
	"app in (a", "app in a", "app > x", "!app=web", "app==", "=web", "app (a)",
	"app in (a, b)", "tier notin (x),env", "!canary", "replicas>3", "x<-5", "app=web AND",
	"a.b/c==d", "app=web", "app==web", "app!=web", "app", "!app", "app notin (a)",
	"app=,tier=db", "a.b/c=d, e in (f)", "app = web", "in=x", "notin", "app in (a b)",
	"app>3.5", "app<", "app=web=x", "!", "app!", "app!=", "app in (A_b-1.c)",
	"app=été", "!é", "a=(é", "a in (é)", "a=b,é", "é", "a!é", "a<é", "a>=é", "a==é",
}

func selectors() {
	r := rand.New(rand.NewSource(3))
	keys := []string{"app", "tier", "a.b/c", "example.com/name", "Env", "x", "k8s.io/ver", "ñ"}
	values := []string{"web", "db", "", "v1.2", "Prod", "-x", "1", "42", "a_b", "été"}
	requirement := func() []string {
		k := keys[r.Intn(len(keys))]
		v := values[r.Intn(len(values))]
		switch r.Intn(7) {
		case 0:
			return []string{k}
		case 1:
			return []string{"!", k}
		case 2:
			return []string{k, []string{"=", "==", "!="}[r.Intn(3)], v}
		case 3, 4:
			op := []string{"in", "notin"}[r.Intn(2)]
			set := []string{"("}
			for i := 0; i < 1+r.Intn(3); i++ {
				if i > 0 {
					set = append(set, ",")
				}
				set = append(set, values[r.Intn(len(values))])
			}
			return append([]string{k, op}, append(set, ")")...)
		default:
			return []string{k, []string{">", "<"}[r.Intn(2)], []string{"3", "-5", "x", "07"}[r.Intn(4)]}
		}
	}
	inserts := []string{",", "(", ")", "!", "=", "in", " "}
	inputs := append([]string{}, fixedSelectors...)
	for i := 0; i < 2000; i++ {
		var tokens []string
		for j := 0; j < 1+r.Intn(3); j++ {
			if j > 0 {
				tokens = append(tokens, ",")
			}
			tokens = append(tokens, requirement()...)
		}
		switch r.Intn(8) {
		case 0, 1, 2, 3: // valid, untouched
		case 4:
			k := r.Intn(len(tokens))
			tokens = append(tokens[:k], tokens[k+1:]...)
		case 5:
			k := r.Intn(len(tokens))
			tokens = append(tokens[:k+1], tokens[k:]...)
		case 6:
			a, b := r.Intn(len(tokens)), r.Intn(len(tokens))
			tokens[a], tokens[b] = tokens[b], tokens[a]
		default:
			k := r.Intn(len(tokens) + 1)
			tokens = append(tokens[:k], append([]string{inserts[r.Intn(len(inserts))]}, tokens[k:]...)...)
		}
		// Spaces between words; around symbols, sometimes none, as people write.
		var b strings.Builder
		for k, t := range tokens {
			if k > 0 {
				symbol := func(x string) bool { return x != "" && strings.ContainsAny(x[:1], "=!(),<>") }
				if (symbol(t) || symbol(tokens[k-1])) && r.Intn(2) == 0 {
					// no space
				} else {
					b.WriteString(" ")
				}
			}
			b.WriteString(t)
		}
		inputs = append(inputs, b.String())
	}
	for _, s := range inputs {
		verdict := "ok"
		if _, err := labels.Parse(s); err != nil {
			verdict = "err"
		}
		fmt.Printf("%s\t%s\n", s, verdict)
	}
}
